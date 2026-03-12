use std::sync::Arc;

use crate::channels::traits::{Channel, ChannelMessage};

use super::super::super::super::parsing::SessionPartitionCommand;
use super::super::super::super::replies::{
    format_session_partition_admin_required, format_session_partition_admin_required_json,
    format_session_partition_error_json, format_session_partition_status,
    format_session_partition_status_json, format_session_partition_updated,
    format_session_partition_updated_json,
};
use super::super::events::{
    EVENT_DISCORD_COMMAND_SESSION_PARTITION_JSON_REPLIED,
    EVENT_DISCORD_COMMAND_SESSION_PARTITION_REPLIED,
};
use super::super::send::send_response;

pub(super) async fn handle_session_partition(
    channel: &Arc<dyn Channel>,
    msg: &ChannelMessage,
    command: SessionPartitionCommand,
) {
    let command_event = if command.format.is_json() {
        EVENT_DISCORD_COMMAND_SESSION_PARTITION_JSON_REPLIED
    } else {
        EVENT_DISCORD_COMMAND_SESSION_PARTITION_REPLIED
    };
    let current_mode = channel
        .session_partition_mode()
        .unwrap_or_else(|| "unknown".to_string());
    let sender_is_admin = channel.is_authorized_for_control_command(&msg.sender, &msg.content);
    if !sender_is_admin {
        let response = if command.format.is_json() {
            format_session_partition_admin_required_json(&msg.sender, &current_mode)
        } else {
            format_session_partition_admin_required(&msg.sender, &current_mode)
        };
        send_response(channel, &msg.recipient, response, msg, command_event).await;
        return;
    }

    let response = match command.mode {
        None if command.format.is_json() => format_session_partition_status_json(&current_mode),
        None => format_session_partition_status(&current_mode),
        Some(mode) => {
            let requested_mode = mode.to_string();
            match channel.set_session_partition_mode(&requested_mode) {
                Ok(()) => {
                    let updated_mode = channel
                        .session_partition_mode()
                        .unwrap_or_else(|| requested_mode.clone());
                    if command.format.is_json() {
                        format_session_partition_updated_json(&requested_mode, &updated_mode)
                    } else {
                        format_session_partition_updated(&requested_mode, &updated_mode)
                    }
                }
                Err(error) if command.format.is_json() => {
                    format_session_partition_error_json(&requested_mode, &error.to_string())
                }
                Err(error) => format!(
                    "Failed to update session partition mode.\nrequested_mode={requested_mode}\nerror={error}"
                ),
            }
        }
    };
    send_response(channel, &msg.recipient, response, msg, command_event).await;
}
