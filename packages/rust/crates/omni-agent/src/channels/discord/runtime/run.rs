use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

use super::super::channel::DiscordControlCommandPolicy;
use super::DiscordRuntimeConfig;
use super::ForegroundInterruptController;
use super::dispatch::process_discord_message_with_interrupt;
use super::ingress::{
    DiscordIngressApp, build_discord_ingress_app_with_partition_and_control_command_policy,
};
use super::managed::push_background_completion;
use crate::agent::Agent;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::{JobManager, JobManagerConfig, TurnRunner};

/// Run Discord channel via HTTP ingress endpoint.
///
/// # Errors
/// Returns an error when channel/runtime initialization fails.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub async fn run_discord_ingress(
    agent: Arc<Agent>,
    bot_token: String,
    allowed_users: Vec<String>,
    allowed_guilds: Vec<String>,
    control_command_policy: DiscordControlCommandPolicy,
    bind_addr: &str,
    ingress_path: &str,
    secret_token: Option<String>,
    runtime_config: DiscordRuntimeConfig,
) -> Result<()> {
    let inbound_queue_capacity = runtime_config.inbound_queue_capacity;
    let turn_timeout_secs = runtime_config.turn_timeout_secs;
    let foreground_max_in_flight_messages = runtime_config.foreground_max_in_flight_messages;

    let (tx, mut inbound_rx) = mpsc::channel::<ChannelMessage>(inbound_queue_capacity);
    let runner: Arc<dyn TurnRunner> = agent.clone();
    let (job_manager, mut completion_rx) = JobManager::start(runner, JobManagerConfig::default());
    let interrupt_controller = ForegroundInterruptController::default();
    let foreground_gate = Arc::new(Semaphore::new(foreground_max_in_flight_messages));
    let mut foreground_tasks = JoinSet::new();
    let ingress = build_discord_ingress_app_with_partition_and_control_command_policy(
        bot_token,
        allowed_users,
        allowed_guilds,
        control_command_policy,
        ingress_path,
        secret_token,
        runtime_config.session_partition,
        tx,
    )?;
    let DiscordIngressApp { app, channel, path } = ingress;
    let channel_for_send: Arc<dyn Channel> = channel.clone();
    let listener = TcpListener::bind(bind_addr).await?;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let mut ingress_server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    println!("Discord ingress listening on {bind_addr}{path} (Ctrl+C to stop)");
    println!("Discord session partition: {}", channel.session_partition());
    println!(
        "Discord foreground config: inbound_queue={inbound_queue_capacity} max_in_flight={foreground_max_in_flight_messages} timeout={turn_timeout_secs}s"
    );
    println!("Background commands: /bg <prompt>, /job <id> [json], /jobs [json]");
    println!(
        "Session commands: /help [json], /session [json], /session budget [json], /session memory [json], /session feedback up|down [json], /session partition [mode|on|off] [json], /session admin [list|set|add|remove|clear] [json], /session inject [status|clear|<qa>...</qa>] [json], /feedback up|down [json], /reset, /clear, /resume, /resume drop, /stop"
    );

    loop {
        tokio::select! {
            maybe_msg = inbound_rx.recv() => {
                let Some(msg) = maybe_msg else {
                    break;
                };
                let gate = Arc::clone(&foreground_gate);
                let agent = Arc::clone(&agent);
                let channel = Arc::clone(&channel_for_send);
                let job_manager = Arc::clone(&job_manager);
                let interrupt_controller = interrupt_controller.clone();
                foreground_tasks.spawn(async move {
                    let Ok(_permit) = gate.acquire_owned().await else {
                        return;
                    };
                    process_discord_message_with_interrupt(
                        agent,
                        channel,
                        msg,
                        &job_manager,
                        turn_timeout_secs,
                        &interrupt_controller,
                    )
                    .await;
                });
            }
            maybe_completion = completion_rx.recv() => {
                let Some(completion) = maybe_completion else {
                    continue;
                };
                push_background_completion(&channel_for_send, completion).await;
            }
            task = foreground_tasks.join_next(), if !foreground_tasks.is_empty() => {
                if let Some(Err(error)) = task {
                    tracing::warn!(error = %error, "discord foreground worker task join error");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Shutting down...");
                break;
            }
            result = &mut ingress_server => {
                match result {
                    Ok(Ok(())) => tracing::warn!("discord ingress server exited"),
                    Ok(Err(error)) => tracing::error!("discord ingress server failed: {error}"),
                    Err(error) => tracing::error!("discord ingress task join error: {error}"),
                }
                break;
            }
        }
    }

    foreground_tasks.abort_all();
    while let Some(result) = foreground_tasks.join_next().await {
        if let Err(error) = result {
            tracing::warn!(error = %error, "discord foreground worker task join error");
        }
    }

    let _ = shutdown_tx.send(());
    Ok(())
}
