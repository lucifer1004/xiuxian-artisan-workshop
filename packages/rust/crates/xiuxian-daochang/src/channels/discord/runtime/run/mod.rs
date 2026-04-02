use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

use crate::agent::Agent;
use crate::channels::discord::channel::DiscordControlCommandPolicy;
use crate::channels::discord::runtime::DiscordRuntimeConfig;
use crate::channels::discord::runtime::foreground::build_foreground_runtime;
use crate::channels::discord::runtime::ingress::{
    DiscordIngressApp, DiscordIngressBuildRequest,
    build_discord_ingress_app_with_partition_and_control_command_policy,
};
use crate::channels::discord::runtime::telemetry::{
    emit_runtime_snapshot, snapshot_interval_from_env,
};
use crate::channels::traits::{Channel, ChannelMessage};

/// Parameters to run Discord HTTP ingress runtime.
#[derive(Debug)]
pub struct DiscordIngressRunRequest {
    /// Bot token used by outbound Discord API calls.
    pub bot_token: String,
    /// Optional allowlist of user ids.
    pub allowed_users: Vec<String>,
    /// Optional allowlist of guild ids.
    pub allowed_guilds: Vec<String>,
    /// Policy for control and slash managed commands.
    pub control_command_policy: DiscordControlCommandPolicy,
    /// TCP address for ingress listener.
    pub bind_addr: String,
    /// HTTP path for ingress endpoint.
    pub ingress_path: String,
    /// Optional shared secret token for ingress validation.
    pub secret_token: Option<String>,
}

/// Run Discord channel via HTTP ingress endpoint.
///
/// # Errors
/// Returns an error when channel/runtime initialization fails.
pub async fn run_discord_ingress(
    agent: Arc<Agent>,
    request: DiscordIngressRunRequest,
    runtime_config: DiscordRuntimeConfig,
) -> Result<()> {
    let DiscordIngressRunRequest {
        bot_token,
        allowed_users,
        allowed_guilds,
        control_command_policy,
        bind_addr,
        ingress_path,
        secret_token,
    } = request;
    let DiscordRuntimeConfig {
        session_partition,
        inbound_queue_capacity,
        turn_timeout_secs,
        foreground_max_in_flight_messages,
        foreground_queue_mode,
    } = runtime_config;

    let (tx, mut inbound_rx) = mpsc::channel::<ChannelMessage>(inbound_queue_capacity);
    let inbound_snapshot_tx = tx.clone();
    let ingress = build_discord_ingress_app_with_partition_and_control_command_policy(
        DiscordIngressBuildRequest {
            bot_token,
            allowed_users,
            allowed_guilds,
            control_command_policy,
            ingress_path,
            secret_token,
            session_partition,
            tx,
        },
    )?;
    let DiscordIngressApp { app, channel, path } = ingress;
    let channel_for_send: Arc<dyn Channel> = channel.clone();
    let (mut runtime, mut completion_rx) = build_foreground_runtime(
        agent,
        channel_for_send,
        turn_timeout_secs,
        foreground_max_in_flight_messages,
        foreground_queue_mode,
    );
    let mut snapshot_tick = snapshot_interval_from_env().map(|period| {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    });
    if let Some(interval) = snapshot_tick.as_mut() {
        let _ = interval.tick().await;
    }
    let listener = TcpListener::bind(&bind_addr).await?;

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
        "Discord foreground config: inbound_queue={inbound_queue_capacity} max_in_flight={foreground_max_in_flight_messages} timeout={turn_timeout_secs}s queue_mode={foreground_queue_mode}"
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
                runtime.spawn_foreground_turn(msg).await;
            }
            maybe_completion = completion_rx.recv() => {
                let Some(completion) = maybe_completion else {
                    continue;
                };
                runtime.push_completion(completion).await;
            }
            () = runtime.join_next_foreground_task(), if runtime.has_foreground_tasks() => {
            }
            _ = async {
                if let Some(interval) = snapshot_tick.as_mut() {
                    let _ = interval.tick().await;
                }
            }, if snapshot_tick.is_some() => {
                let foreground_snapshot = runtime.snapshot();
                emit_runtime_snapshot(
                    "ingress",
                    &inbound_snapshot_tx,
                    inbound_queue_capacity,
                    &foreground_snapshot,
                    runtime.admission_runtime_snapshot(),
                );
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

    runtime.abort_and_drain_foreground_tasks().await;

    let _ = shutdown_tx.send(());
    Ok(())
}
