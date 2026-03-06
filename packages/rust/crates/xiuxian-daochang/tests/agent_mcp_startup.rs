//! Top-level integration tests for `agent::mcp_startup`.

use xiuxian_daochang::AgentConfig;
use xiuxian_daochang::test_support::startup_connect_config;

#[test]
fn startup_connect_config_keeps_runtime_values_in_strict_mode() {
    let config = AgentConfig {
        mcp_pool_size: 8,
        mcp_handshake_timeout_secs: 45,
        mcp_connect_retries: 4,
        mcp_connect_retry_backoff_ms: 2_000,
        mcp_tool_timeout_secs: 90,
        mcp_list_tools_cache_ttl_ms: 2_500,
        ..Default::default()
    };

    let connect = startup_connect_config(&config, true);
    assert_eq!(connect.pool_size, 8);
    assert_eq!(connect.handshake_timeout_secs, 45);
    assert_eq!(connect.connect_retries, 4);
    assert_eq!(connect.connect_retry_backoff_ms, 2_000);
    assert_eq!(connect.tool_timeout_secs, 90);
    assert_eq!(connect.list_tools_cache_ttl_ms, 2_500);
}

#[test]
fn startup_connect_config_clamps_for_non_strict_mode() {
    let config = AgentConfig {
        mcp_pool_size: 4,
        mcp_handshake_timeout_secs: 120,
        mcp_connect_retries: 9,
        mcp_connect_retry_backoff_ms: 0,
        mcp_tool_timeout_secs: 180,
        mcp_list_tools_cache_ttl_ms: 1_000,
        ..Default::default()
    };

    let connect = startup_connect_config(&config, false);
    assert_eq!(connect.pool_size, 4);
    assert_eq!(connect.handshake_timeout_secs, 5);
    assert_eq!(connect.connect_retries, 1);
    assert_eq!(connect.connect_retry_backoff_ms, 1);
    assert_eq!(connect.tool_timeout_secs, 180);
    assert_eq!(connect.list_tools_cache_ttl_ms, 1_000);
}
