use std::sync::Arc;

use crate::agent::Agent;
use crate::channels::telegram::commands::is_stop_command;
use crate::channels::telegram::runtime::dispatch::ForegroundInterruptController;
use crate::channels::traits::{Channel, ChannelMessage};

use super::super::super::observability::send_with_observability;
use super::{
    EVENT_TELEGRAM_COMMAND_SESSION_STOP_IDLE_REPLIED, EVENT_TELEGRAM_COMMAND_SESSION_STOP_REPLIED,
};

pub(in crate::channels::telegram::runtime::jobs) async fn try_handle_stop_command(
    msg: &ChannelMessage,
    channel: &Arc<dyn Channel>,
    agent: &Arc<Agent>,
    interrupt_controller: &ForegroundInterruptController,
    session_id: &str,
) -> bool {
    if !is_stop_command(&msg.content) {
        return false;
    }

    let interrupted = interrupt_controller.interrupt(session_id);
    if interrupted
        && let Err(error) = agent
            .append_turn_for_session(
                session_id,
                "[control] /stop",
                "[system] Current foreground generation interrupted by user request.",
            )
            .await
    {
        tracing::warn!(
            session_id = %session_id,
            error = %error,
            "failed to persist stop-interrupted marker for session"
        );
    }

    let (response, event_name) = if interrupted {
        (
            "Stop signal sent. Current foreground generation is being interrupted.",
            EVENT_TELEGRAM_COMMAND_SESSION_STOP_REPLIED,
        )
    } else {
        (
            "No active foreground generation to stop in this session.",
            EVENT_TELEGRAM_COMMAND_SESSION_STOP_IDLE_REPLIED,
        )
    };

    send_with_observability(
        channel,
        response,
        &msg.recipient,
        "Failed to send stop response",
        Some(event_name),
        Some(&msg.session_key),
    )
    .await;

    true
}
