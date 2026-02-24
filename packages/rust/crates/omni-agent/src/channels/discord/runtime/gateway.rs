use std::sync::Arc;

use anyhow::Result;
use serenity::all::{GatewayIntents, Message};
use serenity::client::{Client, Context, EventHandler};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

use super::super::channel::{DiscordChannel, DiscordControlCommandPolicy};
use super::DiscordRuntimeConfig;
use super::ForegroundInterruptController;
use super::dispatch::process_discord_message_with_interrupt;
use super::managed::push_background_completion;
use crate::agent::Agent;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::{JobManager, JobManagerConfig, TurnRunner};

struct DiscordGatewayEventHandler {
    channel: Arc<DiscordChannel>,
    tx: mpsc::Sender<ChannelMessage>,
}

#[serenity::async_trait]
impl EventHandler for DiscordGatewayEventHandler {
    async fn message(&self, _ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }
        let payload = match serde_json::to_value(&message) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(error = %error, "failed to serialize discord gateway message");
                return;
            }
        };
        let Some(parsed) = self.channel.parse_gateway_message(&payload) else {
            return;
        };
        if self.tx.send(parsed).await.is_err() {
            tracing::warn!("discord inbound queue unavailable");
        }
    }
}

/// Run Discord channel via serenity gateway event stream.
///
/// # Errors
/// Returns an error when channel/runtime initialization fails.
#[allow(clippy::too_many_lines)]
pub async fn run_discord_gateway(
    agent: Arc<Agent>,
    bot_token: String,
    allowed_users: Vec<String>,
    allowed_guilds: Vec<String>,
    control_command_policy: DiscordControlCommandPolicy,
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
    let channel = Arc::new(
        DiscordChannel::new_with_partition_and_control_command_policy(
            bot_token.clone(),
            allowed_users,
            allowed_guilds,
            control_command_policy,
            runtime_config.session_partition,
        )?,
    );
    let channel_for_send: Arc<dyn Channel> = channel.clone();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;
    let handler = DiscordGatewayEventHandler {
        channel: Arc::clone(&channel),
        tx,
    };
    let mut client = Client::builder(bot_token, intents)
        .event_handler(handler)
        .await?;
    let shard_manager = client.shard_manager.clone();
    let mut gateway_task = tokio::spawn(async move { client.start().await });

    println!("Discord gateway connected (Ctrl+C to stop)");
    println!("Discord session partition: {}", channel.session_partition());
    println!(
        "Discord foreground config: inbound_queue={inbound_queue_capacity} max_in_flight={foreground_max_in_flight_messages} timeout={turn_timeout_secs}s"
    );
    println!("Background commands: /bg <prompt>, /job <id> [json], /jobs [json]");
    println!(
        "Session commands: /help [json], /session [json], /session budget [json], /session memory [json], /session feedback up|down [json], /session partition [mode|on|off] [json], /session admin [list|set|add|remove|clear] [json], /session inject [status|clear|<qa>...</qa>] [json], /feedback up|down [json], /reset, /clear, /resume, /resume drop, /stop"
    );

    let mut shutdown_requested = false;

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
                shutdown_requested = true;
                break;
            }
            result = &mut gateway_task => {
                match result {
                    Ok(Ok(())) => tracing::warn!("discord gateway client exited"),
                    Ok(Err(error)) => tracing::error!("discord gateway client failed: {error}"),
                    Err(error) => tracing::error!("discord gateway task join error: {error}"),
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

    if shutdown_requested {
        shard_manager.shutdown_all().await;
        if let Err(error) = gateway_task.await {
            tracing::error!("discord gateway task join error during shutdown: {error}");
        }
    }

    Ok(())
}
