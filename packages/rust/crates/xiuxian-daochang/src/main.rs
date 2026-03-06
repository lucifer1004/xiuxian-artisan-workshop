//! xiuxian-daochang CLI: gateway, stdio, or repl mode.
//!
//! MCP servers from mcp.json only (default `.mcp.json`). Override with `--mcp-config <path>`.
//!
//! Logging: use `-v/--log-verbose`, `--log-format`, `--log-level`, or `RUST_LOG`.

#![recursion_limit = "256"]

mod cli;
mod nodes;
mod resolve;

use clap::Parser;
use xiuxian_logging::init_from_cli;

use xiuxian_daochang::{RuntimeSettings, load_runtime_settings, set_config_home_override};

use crate::cli::{Cli, Command};
use crate::nodes::{
    ChannelCommandRequest, ScheduleModeRequest, run_channel_command, run_embedding_warmup,
    run_gateway_mode, run_repl_mode, run_schedule_mode, run_stdio_mode,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(conf_dir) = cli.conf.clone() {
        set_config_home_override(conf_dir);
    }
    let runtime_settings = load_runtime_settings();
    init_from_cli("xiuxian_daochang", &cli.logging)?;
    Box::pin(dispatch_command(cli.command, &runtime_settings)).await
}

async fn dispatch_command(
    command: Command,
    runtime_settings: &RuntimeSettings,
) -> anyhow::Result<()> {
    match command {
        cmd @ Command::Gateway { .. } => dispatch_gateway_command(runtime_settings, cmd).await,
        cmd @ Command::Stdio { .. } => dispatch_stdio_command(runtime_settings, cmd).await,
        cmd @ Command::Repl { .. } => dispatch_repl_command(runtime_settings, cmd).await,
        cmd @ Command::Schedule { .. } => dispatch_schedule_command(runtime_settings, cmd).await,
        cmd @ Command::Channel { .. } => dispatch_channel_command(runtime_settings, cmd).await,
        Command::EmbeddingWarmup {
            text,
            model,
            mistral_sdk_only,
        } => run_embedding_warmup(runtime_settings, text, model, mistral_sdk_only).await,
    }
}

async fn dispatch_gateway_command(
    runtime_settings: &RuntimeSettings,
    command: Command,
) -> anyhow::Result<()> {
    let Command::Gateway {
        bind,
        turn_timeout,
        max_concurrent,
        mcp_config,
    } = command
    else {
        unreachable!("dispatch_gateway_command expects Command::Gateway")
    };
    run_gateway_mode(
        bind,
        turn_timeout,
        max_concurrent,
        mcp_config,
        runtime_settings,
    )
    .await
}

async fn dispatch_stdio_command(
    runtime_settings: &RuntimeSettings,
    command: Command,
) -> anyhow::Result<()> {
    let Command::Stdio {
        session_id,
        mcp_config,
    } = command
    else {
        unreachable!("dispatch_stdio_command expects Command::Stdio")
    };
    Box::pin(run_stdio_mode(session_id, mcp_config, runtime_settings)).await
}

async fn dispatch_repl_command(
    runtime_settings: &RuntimeSettings,
    command: Command,
) -> anyhow::Result<()> {
    let Command::Repl {
        query,
        session_id,
        mcp_config,
    } = command
    else {
        unreachable!("dispatch_repl_command expects Command::Repl")
    };
    Box::pin(run_repl_mode(
        query,
        session_id,
        mcp_config,
        runtime_settings,
    ))
    .await
}

async fn dispatch_schedule_command(
    runtime_settings: &RuntimeSettings,
    command: Command,
) -> anyhow::Result<()> {
    let Command::Schedule {
        prompt,
        interval_secs,
        max_runs,
        schedule_id,
        session_prefix,
        recipient,
        wait_for_completion_secs,
        mcp_config,
    } = command
    else {
        unreachable!("dispatch_schedule_command expects Command::Schedule")
    };
    run_schedule_mode(ScheduleModeRequest {
        prompt,
        interval_secs,
        max_runs,
        schedule_id,
        session_prefix,
        recipient,
        wait_for_completion_secs,
        mcp_config_path: mcp_config,
        runtime_settings,
    })
    .await
}

async fn dispatch_channel_command(
    runtime_settings: &RuntimeSettings,
    command: Command,
) -> anyhow::Result<()> {
    let Command::Channel {
        provider,
        bot_token,
        mcp_config,
        mode,
        webhook_bind,
        webhook_path,
        webhook_secret_token,
        session_partition,
        inbound_queue_capacity,
        turn_timeout_secs,
        discord_runtime_mode,
        webhook_dedup_backend,
        valkey_url,
        webhook_dedup_ttl_secs,
        webhook_dedup_key_prefix,
    } = command
    else {
        unreachable!("dispatch_channel_command expects Command::Channel")
    };
    Box::pin(run_channel_command(
        ChannelCommandRequest {
            provider,
            bot_token,
            mcp_config,
            mode,
            webhook_bind,
            webhook_path,
            webhook_secret_token,
            session_partition,
            inbound_queue_capacity,
            turn_timeout_secs,
            discord_runtime_mode,
            webhook_dedup_backend,
            valkey_url,
            webhook_dedup_ttl_secs,
            webhook_dedup_key_prefix,
        },
        runtime_settings,
    ))
    .await
}
