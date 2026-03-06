use std::sync::Arc;

use crate::channels::managed_commands::SLASH_SCOPE_JOBS_SUMMARY;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;

use super::super::super::super::parsing::CommandOutputFormat;
use super::super::super::super::replies::{format_job_metrics, format_job_metrics_json};
use super::super::super::auth::ensure_slash_command_authorized;
use super::super::super::events::{
    EVENT_DISCORD_COMMAND_JOBS_SUMMARY_JSON_REPLIED, EVENT_DISCORD_COMMAND_JOBS_SUMMARY_REPLIED,
};
use super::super::super::send::send_response;

pub(in super::super) async fn handle_jobs_summary(
    channel: &Arc<dyn Channel>,
    msg: &ChannelMessage,
    job_manager: &Arc<JobManager>,
    format: CommandOutputFormat,
) {
    if !ensure_slash_command_authorized(channel, msg, SLASH_SCOPE_JOBS_SUMMARY, "/jobs").await {
        return;
    }
    let command_event = if format.is_json() {
        EVENT_DISCORD_COMMAND_JOBS_SUMMARY_JSON_REPLIED
    } else {
        EVENT_DISCORD_COMMAND_JOBS_SUMMARY_REPLIED
    };
    let metrics = job_manager.metrics().await;
    let response = if format.is_json() {
        format_job_metrics_json(&metrics)
    } else {
        format_job_metrics(&metrics)
    };
    send_response(channel, &msg.recipient, response, msg, command_event).await;
}
