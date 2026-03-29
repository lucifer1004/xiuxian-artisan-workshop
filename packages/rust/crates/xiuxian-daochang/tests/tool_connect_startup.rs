#![allow(
    missing_docs,
    unused_imports,
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::field_reassign_with_default,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::map_unwrap_or,
    clippy::option_as_ref_deref,
    clippy::unreadable_literal,
    clippy::useless_conversion,
    clippy::match_wildcard_for_single_variants,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_raw_string_hashes,
    clippy::manual_async_fn,
    clippy::manual_let_else,
    clippy::too_many_lines,
    clippy::unnecessary_literal_bound,
    clippy::needless_pass_by_value,
    clippy::struct_field_names,
    clippy::single_match_else,
    clippy::assigning_clones
)]

//! Startup external tool connect behavior.

use omni_agent::{Agent, AgentConfig, ToolServerEntry};

#[tokio::test]
async fn agent_startup_tool_connect_retries_are_applied() {
    let config = AgentConfig {
        tool_servers: vec![ToolServerEntry {
            name: "local-unreachable".to_string(),
            url: Some("http://127.0.0.1:1/sse".to_string()),
            command: None,
            args: None,
        }],
        tool_pool_size: 1,
        tool_handshake_timeout_secs: 1,
        tool_connect_retries: 2,
        tool_strict_startup: true,
        tool_connect_retry_backoff_ms: 10,
        ..Default::default()
    };

    let error = match Agent::from_config(config).await {
        Ok(_) => panic!("startup should fail for unreachable tool endpoint"),
        Err(error) => error,
    };
    let message = format!("{error:#}");
    assert!(
        message.contains("connect failed after 2 attempts"),
        "unexpected error message: {message}"
    );
    assert!(
        message.contains("http://127.0.0.1:1/sse"),
        "unexpected error message: {message}"
    );
}

#[tokio::test]
async fn agent_startup_non_strict_tool_connect_failure_continues() {
    let config = AgentConfig {
        tool_servers: vec![ToolServerEntry {
            name: "local-unreachable".to_string(),
            url: Some("http://127.0.0.1:1/sse".to_string()),
            command: None,
            args: None,
        }],
        tool_pool_size: 1,
        tool_handshake_timeout_secs: 30,
        tool_connect_retries: 3,
        tool_strict_startup: false,
        tool_connect_retry_backoff_ms: 1_000,
        ..Default::default()
    };

    let built = Agent::from_config(config).await;
    assert!(
        built.is_ok(),
        "non-strict startup should continue when tool runtime is unavailable"
    );
}
