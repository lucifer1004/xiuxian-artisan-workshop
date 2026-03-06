//! Telegram runtime wiring (polling/webhook + foreground/background execution).

mod console;
mod dispatch;
pub(crate) mod jobs;
mod run_polling;
mod run_webhook;
mod telemetry;
mod webhook;

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::agent::Agent;
use crate::channels::managed_runtime::ForegroundQueueMode;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;

pub(crate) use dispatch::ForegroundInterruptController;
pub use run_polling::{run_telegram, run_telegram_with_control_command_policy};
pub use run_webhook::{
    TelegramWebhookPolicyRunRequest, TelegramWebhookRunRequest, run_telegram_webhook,
    run_telegram_webhook_with_control_command_policy,
};
pub use webhook::{
    TelegramWebhookApp, TelegramWebhookControlPolicyBuildRequest,
    TelegramWebhookPartitionBuildRequest, build_telegram_webhook_app,
    build_telegram_webhook_app_with_control_command_policy,
    build_telegram_webhook_app_with_partition,
};

pub(crate) async fn test_handle_inbound_message_with_interrupt(
    msg: ChannelMessage,
    channel: &Arc<dyn Channel>,
    foreground_tx: &mpsc::Sender<ChannelMessage>,
    interrupt_controller: &ForegroundInterruptController,
    job_manager: &Arc<JobManager>,
    agent: &Arc<Agent>,
    queue_mode: ForegroundQueueMode,
) -> bool {
    jobs::handle_inbound_message_with_interrupt(
        msg,
        channel,
        foreground_tx,
        interrupt_controller,
        job_manager,
        agent,
        queue_mode,
    )
    .await
}

pub(crate) async fn test_push_background_completion(
    channel: &Arc<dyn Channel>,
    agent: &Arc<Agent>,
    completion: crate::jobs::JobCompletion,
) {
    jobs::push_background_completion(channel, agent, completion).await;
}

pub(crate) fn test_resolve_snapshot_interval_secs<F>(lookup: F) -> Option<u64>
where
    F: Fn(&str) -> Option<String>,
{
    telemetry::resolve_snapshot_interval_secs(lookup)
}

#[must_use]
pub(crate) fn test_log_preview(s: &str) -> String {
    jobs::observability::preview::log_preview(s)
}
