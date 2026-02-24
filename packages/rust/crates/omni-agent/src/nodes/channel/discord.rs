use std::sync::Arc;

use omni_agent::{
    DiscordCommandAdminRule, DiscordControlCommandPolicy, DiscordRuntimeConfig,
    DiscordSessionPartition, DiscordSlashCommandPolicy, RuntimeSettings,
    build_discord_acl_overrides, run_discord_gateway, run_discord_ingress,
};

use crate::agent_builder::build_agent;
use crate::cli::DiscordRuntimeMode;
use crate::resolve::{
    resolve_discord_runtime_mode, resolve_positive_u64, resolve_positive_usize, resolve_string,
};

use super::ChannelCommandRequest;
use super::common::{log_control_command_allow_override, log_slash_command_allow_override};

const DISCORD_DEFAULT_INBOUND_QUEUE_CAPACITY: usize = 512;
const DISCORD_DEFAULT_TURN_TIMEOUT_SECS: u64 = 120;
const DISCORD_DEFAULT_FOREGROUND_MAX_IN_FLIGHT_MESSAGES: usize = 16;
const DISCORD_DEFAULT_INGRESS_BIND: &str = "0.0.0.0:8082";
const DISCORD_DEFAULT_INGRESS_PATH: &str = "/discord/ingress";

#[allow(clippy::similar_names, clippy::too_many_lines)]
pub(super) async fn run_discord_channel_command(
    req: ChannelCommandRequest,
    runtime_settings: &RuntimeSettings,
) -> anyhow::Result<()> {
    let ChannelCommandRequest {
        bot_token,
        mcp_config,
        session_partition,
        inbound_queue_capacity,
        turn_timeout_secs,
        discord_runtime_mode,
        ..
    } = req;

    let token = bot_token
        .or_else(|| std::env::var("DISCORD_BOT_TOKEN").ok())
        .ok_or_else(|| anyhow::anyhow!("--bot-token or DISCORD_BOT_TOKEN required"))?;
    let acl_overrides = build_discord_acl_overrides(runtime_settings)?;
    let allowed_users = acl_overrides.allowed_users;
    let allowed_guilds = acl_overrides.allowed_guilds;
    let admin_users = acl_overrides
        .admin_users
        .unwrap_or_else(|| allowed_users.clone());
    let control_command_allow_from = acl_overrides.control_command_allow_from;
    let control_command_rules = acl_overrides.control_command_rules;
    let slash_command_allow_from = acl_overrides.slash_command_allow_from;
    let slash_session_status_allow_from = acl_overrides.slash_session_status_allow_from;
    let slash_session_budget_allow_from = acl_overrides.slash_session_budget_allow_from;
    let slash_session_memory_allow_from = acl_overrides.slash_session_memory_allow_from;
    let slash_session_feedback_allow_from = acl_overrides.slash_session_feedback_allow_from;
    let slash_job_allow_from = acl_overrides.slash_job_allow_from;
    let slash_jobs_allow_from = acl_overrides.slash_jobs_allow_from;
    let slash_bg_allow_from = acl_overrides.slash_bg_allow_from;
    let raw_session_partition = resolve_string(
        session_partition,
        "OMNI_AGENT_DISCORD_SESSION_PARTITION",
        runtime_settings.discord.session_partition.as_deref(),
        "guild_channel_user",
    );
    let session_partition = raw_session_partition
        .parse::<DiscordSessionPartition>()
        .map_err(|_| {
            anyhow::anyhow!("invalid discord session partition mode: {raw_session_partition}")
        })?;
    let inbound_queue_capacity = resolve_positive_usize(
        inbound_queue_capacity,
        "OMNI_AGENT_DISCORD_INBOUND_QUEUE_CAPACITY",
        runtime_settings.discord.inbound_queue_capacity,
        DISCORD_DEFAULT_INBOUND_QUEUE_CAPACITY,
    );
    let turn_timeout_secs = resolve_positive_u64(
        turn_timeout_secs,
        "OMNI_AGENT_DISCORD_TURN_TIMEOUT_SECS",
        runtime_settings.discord.turn_timeout_secs,
        DISCORD_DEFAULT_TURN_TIMEOUT_SECS,
    );
    let foreground_max_in_flight_messages = resolve_positive_usize(
        None,
        "OMNI_AGENT_DISCORD_FOREGROUND_MAX_IN_FLIGHT_MESSAGES",
        runtime_settings.discord.foreground_max_in_flight_messages,
        DISCORD_DEFAULT_FOREGROUND_MAX_IN_FLIGHT_MESSAGES,
    );
    let runtime_mode = resolve_discord_runtime_mode(
        discord_runtime_mode,
        runtime_settings.discord.runtime_mode.as_deref(),
    );
    let ingress_bind = resolve_string(
        None,
        "OMNI_AGENT_DISCORD_INGRESS_BIND",
        runtime_settings.discord.ingress_bind.as_deref(),
        DISCORD_DEFAULT_INGRESS_BIND,
    );
    let ingress_path = resolve_string(
        None,
        "OMNI_AGENT_DISCORD_INGRESS_PATH",
        runtime_settings.discord.ingress_path.as_deref(),
        DISCORD_DEFAULT_INGRESS_PATH,
    );
    let ingress_secret_token = std::env::var("OMNI_AGENT_DISCORD_INGRESS_SECRET_TOKEN")
        .ok()
        .or_else(|| runtime_settings.discord.ingress_secret_token.clone())
        .and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

    run_discord_channel_mode(
        token,
        allowed_users,
        allowed_guilds,
        admin_users,
        control_command_allow_from,
        control_command_rules,
        slash_command_allow_from,
        slash_session_status_allow_from,
        slash_session_budget_allow_from,
        slash_session_memory_allow_from,
        slash_session_feedback_allow_from,
        slash_job_allow_from,
        slash_jobs_allow_from,
        slash_bg_allow_from,
        mcp_config,
        runtime_mode,
        session_partition,
        inbound_queue_capacity,
        turn_timeout_secs,
        foreground_max_in_flight_messages,
        ingress_bind,
        ingress_path,
        ingress_secret_token,
        runtime_settings,
    )
    .await
}

#[allow(clippy::similar_names, clippy::too_many_arguments)]
async fn run_discord_channel_mode(
    bot_token: String,
    allowed_users: Vec<String>,
    allowed_guilds: Vec<String>,
    admin_users: Vec<String>,
    control_command_allow_from: Option<Vec<String>>,
    control_command_rules: Vec<DiscordCommandAdminRule>,
    slash_command_allow_from: Option<Vec<String>>,
    slash_session_status_allow_from: Option<Vec<String>>,
    slash_session_budget_allow_from: Option<Vec<String>>,
    slash_session_memory_allow_from: Option<Vec<String>>,
    slash_session_feedback_allow_from: Option<Vec<String>>,
    slash_job_allow_from: Option<Vec<String>>,
    slash_jobs_allow_from: Option<Vec<String>>,
    slash_bg_allow_from: Option<Vec<String>>,
    mcp_config_path: std::path::PathBuf,
    runtime_mode: DiscordRuntimeMode,
    session_partition: DiscordSessionPartition,
    inbound_queue_capacity: usize,
    turn_timeout_secs: u64,
    foreground_max_in_flight_messages: usize,
    ingress_bind: String,
    ingress_path: String,
    ingress_secret_token: Option<String>,
    runtime_settings: &RuntimeSettings,
) -> anyhow::Result<()> {
    let agent = Arc::new(build_agent(&mcp_config_path, runtime_settings).await?);
    let users = allowed_users;
    let guilds = allowed_guilds;
    let admins = admin_users;
    let control_command_allow_from_entries = control_command_allow_from;
    log_control_command_allow_override("discord", &control_command_allow_from_entries);
    let slash_command_allow_from_entries = slash_command_allow_from;
    log_slash_command_allow_override("discord", &slash_command_allow_from_entries);
    let slash_command_policy = DiscordSlashCommandPolicy {
        slash_command_allow_from: slash_command_allow_from_entries,
        session_status_allow_from: slash_session_status_allow_from,
        session_budget_allow_from: slash_session_budget_allow_from,
        session_memory_allow_from: slash_session_memory_allow_from,
        session_feedback_allow_from: slash_session_feedback_allow_from,
        job_status_allow_from: slash_job_allow_from,
        jobs_summary_allow_from: slash_jobs_allow_from,
        background_submit_allow_from: slash_bg_allow_from,
    };
    let control_command_policy = DiscordControlCommandPolicy::new(
        admins,
        control_command_allow_from_entries,
        control_command_rules,
    )
    .with_slash_command_policy(slash_command_policy);

    if users.is_empty() && guilds.is_empty() {
        tracing::warn!(
            "Discord ACL allowlist is empty; all inbound will be rejected. \
             Configure `discord.acl.allow.users` or `discord.acl.allow.guilds` to allow traffic."
        );
    }

    let runtime_config = DiscordRuntimeConfig {
        session_partition,
        inbound_queue_capacity,
        turn_timeout_secs,
        foreground_max_in_flight_messages,
    };
    match runtime_mode {
        DiscordRuntimeMode::Gateway => {
            run_discord_gateway(
                Arc::clone(&agent),
                bot_token,
                users,
                guilds,
                control_command_policy,
                runtime_config,
            )
            .await
        }
        DiscordRuntimeMode::Ingress => {
            run_discord_ingress(
                Arc::clone(&agent),
                bot_token,
                users,
                guilds,
                control_command_policy,
                &ingress_bind,
                &ingress_path,
                ingress_secret_token,
                runtime_config,
            )
            .await
        }
    }
}
