//! Shared MCP tool error policy helpers.
//!
//! This module centralizes degraded tool-result payload shaping and timeout
//! classification so callers (for example `xiuxian-daochang`) can stay thin.

/// Returns `true` when an error message is timeout-class and should be treated
/// as a non-fatal degraded tool result.
#[must_use]
pub fn is_timeout_error_message(message: &str) -> bool {
    let lowercase = message.to_ascii_lowercase();
    lowercase.contains("timed out")
        || lowercase.contains("timeout")
        || lowercase.contains("mcp.pool.call.waiting")
}

/// Builds a canonical degraded tool-result payload as a JSON string.
///
/// The payload format is intentionally compact and channel-agnostic so higher
/// layers can pass it back to the model as a tool error without terminating
/// the current turn.
#[must_use]
pub fn degraded_tool_error_payload(
    tool: &str,
    source: Option<&str>,
    error_kind: &str,
    timeout_secs: Option<u64>,
    message: &str,
) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert("ok".to_string(), serde_json::Value::Bool(false));
    payload.insert("degraded".to_string(), serde_json::Value::Bool(true));
    payload.insert(
        "tool".to_string(),
        serde_json::Value::String(tool.to_string()),
    );
    payload.insert(
        "error_kind".to_string(),
        serde_json::Value::String(error_kind.to_string()),
    );
    payload.insert(
        "message".to_string(),
        serde_json::Value::String(message.to_string()),
    );
    if let Some(source) = source {
        payload.insert(
            "source".to_string(),
            serde_json::Value::String(source.to_string()),
        );
    }
    if let Some(timeout_secs) = timeout_secs {
        payload.insert(
            "timeout_secs".to_string(),
            serde_json::Value::Number(serde_json::Number::from(timeout_secs)),
        );
    }
    serde_json::Value::Object(payload).to_string()
}

/// Builds the standard timeout-degraded tool payload.
#[must_use]
pub fn timeout_tool_error_payload(source: &str, tool: &str, timeout_secs: u64) -> String {
    let message = format!(
        "Tool `{tool}` from {source} timed out after {timeout_secs}s and was aborted; continuing without tool result."
    );
    degraded_tool_error_payload(tool, Some(source), "timeout", Some(timeout_secs), &message)
}
