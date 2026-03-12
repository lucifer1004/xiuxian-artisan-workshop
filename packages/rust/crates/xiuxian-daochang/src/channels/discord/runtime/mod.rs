//! Discord runtime wiring (ingress + foreground turn execution).

mod config;
mod dispatch;
mod foreground;
mod gateway;
mod ingress;
mod interrupt;
mod managed;
mod run;
mod telemetry;

pub use config::DiscordRuntimeConfig;
pub use gateway::{run_discord_gateway, run_discord_gateway_listener};
pub use ingress::{
    DiscordIngressApp, DiscordIngressBuildRequest, build_discord_ingress_app,
    build_discord_ingress_app_with_control_command_policy,
    build_discord_ingress_app_with_partition_and_control_command_policy,
};
pub use run::{DiscordIngressRunRequest, run_discord_ingress};

use crate::agent::Agent;
use crate::channels::managed_runtime::ForegroundQueueMode;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;
use std::sync::Arc;

pub(crate) use interrupt::ForegroundInterruptController;

pub(crate) async fn test_process_discord_message_with_interrupt(
    agent: Arc<Agent>,
    channel: Arc<dyn Channel>,
    msg: ChannelMessage,
    job_manager: &Arc<JobManager>,
    turn_timeout_secs: u64,
    foreground_queue_mode: ForegroundQueueMode,
    interrupt_controller: &ForegroundInterruptController,
) {
    dispatch::process_discord_message_with_interrupt(
        agent,
        channel,
        msg,
        job_manager,
        turn_timeout_secs,
        foreground_queue_mode,
        interrupt_controller,
    )
    .await;
}

pub(crate) fn test_resolve_snapshot_interval_secs<F>(lookup: F) -> Option<u64>
where
    F: Fn(&str) -> Option<String>,
{
    telemetry::resolve_snapshot_interval_secs(lookup)
}

pub(crate) fn test_interrupted_reply_is_suppressed(
    msg: &ChannelMessage,
    turn_timeout_secs: u64,
) -> bool {
    dispatch::test_interrupted_reply_is_suppressed(msg, turn_timeout_secs)
}
