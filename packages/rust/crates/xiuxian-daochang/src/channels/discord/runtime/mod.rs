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

use std::sync::Arc;

use crate::agent::Agent;
use crate::channels::managed_runtime::ForegroundQueueMode;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;

pub(crate) use interrupt::ForegroundInterruptController;

/// Test-only foreground runtime harness for exercising Discord scheduling behavior.
pub(crate) struct TestDiscordForegroundRuntime {
    inner: foreground::DiscordForegroundRuntime,
}

impl TestDiscordForegroundRuntime {
    pub(crate) async fn spawn_foreground_turn(&mut self, msg: ChannelMessage) {
        self.inner.spawn_foreground_turn(msg).await;
    }

    pub(crate) fn has_foreground_tasks(&self) -> bool {
        self.inner.has_foreground_tasks()
    }

    pub(crate) async fn join_next_foreground_task(&mut self) {
        self.inner.join_next_foreground_task().await;
    }

    pub(crate) async fn abort_and_drain_foreground_tasks(&mut self) {
        self.inner.abort_and_drain_foreground_tasks().await;
    }
}

pub(crate) fn test_build_discord_foreground_runtime(
    agent: Arc<Agent>,
    channel: Arc<dyn Channel>,
    turn_timeout_secs: u64,
    foreground_max_in_flight_messages: usize,
    foreground_queue_mode: ForegroundQueueMode,
) -> TestDiscordForegroundRuntime {
    let (runtime, _completion_rx) = foreground::build_foreground_runtime(
        agent,
        channel,
        turn_timeout_secs,
        foreground_max_in_flight_messages,
        foreground_queue_mode,
    );
    TestDiscordForegroundRuntime { inner: runtime }
}

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

pub(crate) async fn test_push_background_completion(
    channel: &Arc<dyn Channel>,
    agent: &Arc<Agent>,
    completion: crate::jobs::JobCompletion,
) {
    managed::push_background_completion(channel, agent, completion).await;
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
