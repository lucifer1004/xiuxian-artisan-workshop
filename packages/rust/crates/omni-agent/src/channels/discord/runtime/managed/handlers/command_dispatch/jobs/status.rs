use std::sync::Arc;

use crate::channels::managed_commands::SLASH_SCOPE_JOB_STATUS;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;

use super::super::super::super::parsing::CommandOutputFormat;
use super::super::super::super::replies::{
    format_job_not_found, format_job_not_found_json, format_job_status, format_job_status_json,
};
use super::super::super::auth::ensure_slash_command_authorized;
use super::super::super::events::{
    EVENT_DISCORD_COMMAND_JOB_STATUS_JSON_REPLIED, EVENT_DISCORD_COMMAND_JOB_STATUS_REPLIED,
};
use super::super::super::send::send_response;

pub(in super::super) async fn handle_job_status(
    channel: &Arc<dyn Channel>,
    msg: &ChannelMessage,
    job_manager: &Arc<JobManager>,
    job_id: String,
    format: CommandOutputFormat,
) {
    if !ensure_slash_command_authorized(channel, msg, SLASH_SCOPE_JOB_STATUS, "/job").await {
        return;
    }
    let command_event = if format.is_json() {
        EVENT_DISCORD_COMMAND_JOB_STATUS_JSON_REPLIED
    } else {
        EVENT_DISCORD_COMMAND_JOB_STATUS_REPLIED
    };
    let response = match job_manager.get_status(&job_id).await {
        Some(snapshot) if format.is_json() => format_job_status_json(&snapshot),
        Some(snapshot) => format_job_status(&snapshot),
        None if format.is_json() => format_job_not_found_json(&job_id),
        None => format_job_not_found(&job_id),
    };
    send_response(channel, &msg.recipient, response, msg, command_event).await;
}
