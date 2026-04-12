use std::sync::Arc;

use crate::channels::discord::runtime::managed::handlers::events::{
    EVENT_DISCORD_COMMAND_SESSION_MENTION_JSON_REPLIED,
    EVENT_DISCORD_COMMAND_SESSION_MENTION_REPLIED,
};
use crate::channels::discord::runtime::managed::handlers::send::send_response;
use crate::channels::discord::runtime::managed::parsing::{
    SessionMentionCommand, SessionMentionMode,
};
use crate::channels::discord::runtime::managed::replies::{
    format_command_error_json, format_session_mention_admin_required,
    format_session_mention_admin_required_json, format_session_mention_status,
    format_session_mention_status_json, format_session_mention_updated,
    format_session_mention_updated_json,
};
use crate::channels::traits::{Channel, ChannelMessage};

pub(in crate::channels::discord::runtime::managed::handlers::command_dispatch) async fn handle_session_mention(
    channel: &Arc<dyn Channel>,
    msg: &ChannelMessage,
    command: SessionMentionCommand,
) {
    let command_event = if command.format.is_json() {
        EVENT_DISCORD_COMMAND_SESSION_MENTION_JSON_REPLIED
    } else {
        EVENT_DISCORD_COMMAND_SESSION_MENTION_REPLIED
    };

    let current = match channel.recipient_mention_policy_status(&msg.recipient) {
        Ok(status) => status,
        Err(error) if command.format.is_json() => {
            let response = format_command_error_json("session_mention_status", &error.to_string());
            send_response(channel, &msg.recipient, response, msg, command_event).await;
            return;
        }
        Err(error) => {
            let response = format!("Failed to inspect session mention policy: {error}");
            send_response(channel, &msg.recipient, response, msg, command_event).await;
            return;
        }
    };

    let sender_is_admin = channel.is_authorized_for_control_command_for_recipient(
        &msg.sender,
        &msg.content,
        &msg.recipient,
    );

    if !sender_is_admin {
        let response = if command.format.is_json() {
            format_session_mention_admin_required_json(&msg.sender, &msg.recipient, &current)
        } else {
            format_session_mention_admin_required(&msg.sender, &msg.recipient, &current)
        };
        send_response(channel, &msg.recipient, response, msg, command_event).await;
        return;
    }

    let response = match command.mode {
        None if command.format.is_json() => {
            format_session_mention_status_json(&msg.recipient, &current)
        }
        None => format_session_mention_status(&msg.recipient, &current),
        Some(mode) => {
            let requested = match mode {
                SessionMentionMode::Require => Some(true),
                SessionMentionMode::Open => Some(false),
                SessionMentionMode::Inherit => None,
            };
            match channel.set_recipient_require_mention(&msg.recipient, requested) {
                Ok(updated) if command.format.is_json() => {
                    format_session_mention_updated_json(&msg.recipient, mode, &updated)
                }
                Ok(updated) => format_session_mention_updated(&msg.recipient, mode, &updated),
                Err(error) if command.format.is_json() => {
                    format_command_error_json("session_mention_update", &error.to_string())
                }
                Err(error) => format!("Failed to update session mention policy: {error}"),
            }
        }
    };

    send_response(channel, &msg.recipient, response, msg, command_event).await;
}
