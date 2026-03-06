//! Integration tests for shared MCP tool policy helpers.

use xiuxian_mcp::{
    degraded_tool_error_payload, is_timeout_error_message, timeout_tool_error_payload,
};

#[test]
fn timeout_error_message_classification_matches_expected_patterns() {
    assert!(is_timeout_error_message("request timed out"));
    assert!(is_timeout_error_message("hard TIMEOUT while calling tool"));
    assert!(is_timeout_error_message(
        "mcp.pool.call.waiting exceeded guard threshold"
    ));
    assert!(!is_timeout_error_message("connection refused by peer"));
}

#[test]
fn degraded_tool_error_payload_contains_required_fields() {
    let payload = degraded_tool_error_payload(
        "memory.search_memory",
        None,
        "embedding_timeout",
        None,
        "Embedding lookup timed out; continuing.",
    );
    let value: serde_json::Value = match serde_json::from_str(&payload) {
        Ok(value) => value,
        Err(error) => panic!("degraded payload must be valid json: {error}"),
    };
    assert_eq!(value["ok"], serde_json::Value::Bool(false));
    assert_eq!(value["degraded"], serde_json::Value::Bool(true));
    assert_eq!(
        value["tool"],
        serde_json::Value::String("memory.search_memory".to_string())
    );
    assert_eq!(
        value["error_kind"],
        serde_json::Value::String("embedding_timeout".to_string())
    );
    assert_eq!(
        value["message"],
        serde_json::Value::String("Embedding lookup timed out; continuing.".to_string())
    );
    assert!(value.get("source").is_none());
    assert!(value.get("timeout_secs").is_none());
}

#[test]
fn timeout_tool_error_payload_includes_source_and_timeout() {
    let payload = timeout_tool_error_payload("mcp", "bridge.hang", 3);
    let value: serde_json::Value = match serde_json::from_str(&payload) {
        Ok(value) => value,
        Err(error) => panic!("timeout payload must be valid json: {error}"),
    };
    assert_eq!(value["ok"], serde_json::Value::Bool(false));
    assert_eq!(value["degraded"], serde_json::Value::Bool(true));
    assert_eq!(
        value["tool"],
        serde_json::Value::String("bridge.hang".to_string())
    );
    assert_eq!(
        value["source"],
        serde_json::Value::String("mcp".to_string())
    );
    assert_eq!(
        value["error_kind"],
        serde_json::Value::String("timeout".to_string())
    );
    assert_eq!(
        value["timeout_secs"],
        serde_json::Value::Number(serde_json::Number::from(3_u64))
    );
}
