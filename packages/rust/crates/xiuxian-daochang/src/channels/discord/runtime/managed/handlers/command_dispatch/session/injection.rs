use std::sync::Arc;

use serde_json::json;

use crate::agent::Agent;
use crate::channels::traits::{Channel, ChannelMessage};

use super::super::super::super::parsing::{SessionInjectionAction, SessionInjectionCommand};
use super::super::super::super::replies::{
    format_command_error_json, format_control_command_admin_required,
};
use super::super::super::events::{
    EVENT_DISCORD_COMMAND_CONTROL_ADMIN_REQUIRED_REPLIED,
    EVENT_DISCORD_COMMAND_SESSION_INJECTION_JSON_REPLIED,
    EVENT_DISCORD_COMMAND_SESSION_INJECTION_REPLIED,
};
use super::super::super::send::send_response;
use super::helpers::truncate_preview;

pub(in super::super) async fn handle_session_injection(
    agent: &Arc<Agent>,
    channel: &Arc<dyn Channel>,
    msg: &ChannelMessage,
    session_id: &str,
    command: SessionInjectionCommand,
) {
    if !channel.is_authorized_for_control_command_for_recipient(
        &msg.sender,
        &msg.content,
        &msg.recipient,
    ) {
        let response = format_control_command_admin_required("/session inject", &msg.sender);
        send_response(
            channel,
            &msg.recipient,
            response,
            msg,
            EVENT_DISCORD_COMMAND_CONTROL_ADMIN_REQUIRED_REPLIED,
        )
        .await;
        return;
    }

    let command_event = if command.format.is_json() {
        EVENT_DISCORD_COMMAND_SESSION_INJECTION_JSON_REPLIED
    } else {
        EVENT_DISCORD_COMMAND_SESSION_INJECTION_REPLIED
    };

    let response = match command.action {
        SessionInjectionAction::Status => {
            match agent.inspect_session_system_prompt_injection(session_id).await {
                Some(snapshot) if command.format.is_json() => json!({
                    "kind": "session_injection",
                    "configured": true,
                    "qa_count": snapshot.qa_count,
                    "updated_at_unix_ms": snapshot.updated_at_unix_ms,
                    "xml": snapshot.xml,
                })
                .to_string(),
                Some(snapshot) => {
                    let preview = truncate_preview(&snapshot.xml, 800);
                    format!(
                        "Session system prompt injection is configured.\nqa_count={}\nupdated_at_unix_ms={}\nxml_preview:\n{}",
                        snapshot.qa_count, snapshot.updated_at_unix_ms, preview
                    )
                }
                None if command.format.is_json() => json!({
                    "kind": "session_injection",
                    "configured": false,
                    "message": "No system prompt injection is configured for this session.",
                })
                .to_string(),
                None => "No system prompt injection is configured for this session.\nUse `/session inject <qa>...</qa>` to configure it.".to_string(),
            }
        }
        SessionInjectionAction::Clear => match agent
            .clear_session_system_prompt_injection(session_id)
            .await
        {
            Ok(cleared) if command.format.is_json() => json!({
                "kind": "session_injection",
                "cleared": cleared,
            })
            .to_string(),
            Ok(true) => "Session system prompt injection cleared.".to_string(),
            Ok(false) => "No session system prompt injection existed to clear.".to_string(),
            Err(error) if command.format.is_json() => {
                format_command_error_json("session_injection_clear", &error.to_string())
            }
            Err(error) => format!("Failed to clear session system prompt injection: {error}"),
        },
        SessionInjectionAction::SetXml(payload) => {
            match agent
                .upsert_session_system_prompt_injection_xml(session_id, &payload)
                .await
            {
                Ok(snapshot) => {
                    if command.format.is_json() {
                        json!({
                            "kind": "session_injection",
                            "configured": true,
                            "qa_count": snapshot.qa_count,
                            "updated_at_unix_ms": snapshot.updated_at_unix_ms,
                        })
                        .to_string()
                    } else {
                        format!(
                            "Session system prompt injection updated.\nqa_count={}\nupdated_at_unix_ms={}",
                            snapshot.qa_count, snapshot.updated_at_unix_ms
                        )
                    }
                }
                Err(error) if command.format.is_json() => {
                    format_command_error_json("session_injection_set", &error.to_string())
                }
                Err(error) => format!("Invalid system prompt injection payload: {error}"),
            }
        }
    };

    send_response(channel, &msg.recipient, response, msg, command_event).await;
}
