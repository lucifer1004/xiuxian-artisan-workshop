use serde_json::json;

use crate::agent::SessionRecallFeedbackDirection;

pub(crate) fn format_session_feedback(
    direction: SessionRecallFeedbackDirection,
    previous_bias: f32,
    updated_bias: f32,
) -> String {
    let direction_label = match direction {
        SessionRecallFeedbackDirection::Up => "up",
        SessionRecallFeedbackDirection::Down => "down",
    };
    format!(
        "Session recall feedback updated.\ndirection={direction_label}\nprevious_bias={previous_bias:.3}\nupdated_bias={updated_bias:.3}"
    )
}

pub(crate) fn format_session_feedback_json(
    direction: SessionRecallFeedbackDirection,
    previous_bias: f32,
    updated_bias: f32,
) -> String {
    let direction_label = match direction {
        SessionRecallFeedbackDirection::Up => "up",
        SessionRecallFeedbackDirection::Down => "down",
    };
    json!({
        "kind": "session_feedback",
        "applied": true,
        "direction": direction_label,
        "previous_bias": previous_bias,
        "updated_bias": updated_bias,
    })
    .to_string()
}

pub(crate) fn format_session_feedback_unavailable_json() -> String {
    json!({
        "kind": "session_feedback",
        "applied": false,
        "reason": "memory_disabled",
        "message": "Session recall feedback is unavailable because memory is disabled.",
    })
    .to_string()
}

pub(crate) fn format_command_error_json(command: &str, error: &str) -> String {
    json!({
        "kind": "command_error",
        "command": command,
        "status": "error",
        "error": error,
    })
    .to_string()
}
