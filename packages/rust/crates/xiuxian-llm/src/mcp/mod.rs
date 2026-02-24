//! MCP runtime facade shared across xiuxian runtime crates.
//!
//! This module re-exports the stable client surface from `xiuxian-mcp` and
//! hosts transport/connect helpers reused by `omni-agent`.

pub mod config;
pub mod connect;
pub mod discover_cache;
pub mod health;
pub mod pool;
pub mod pool_call;
pub mod pool_core;
pub mod pool_retry;
pub mod pool_utils;
pub mod stats;
pub mod task_handle;
pub mod transport_error;
pub mod wait_heartbeat;
pub mod wait_logger;

pub use config::McpPoolConnectConfig;
pub use connect::connect_one_client_with_retry;
pub use discover_cache::{DiscoverCacheConfig, DiscoverCacheRuntimeInfo, DiscoverReadThroughCache};
pub use health::{HealthProbeStatus, derive_health_url, probe_health_status, probe_health_summary};
pub use pool::{McpClientPool, connect_pool};
pub use pool_call::{call_tool_once, list_tools_once};
pub use pool_core::{connect_pool_clients_with_retry, reconnect_pool_client_with_retry};
pub use pool_retry::{run_tool_call_with_retry, run_tools_list_with_fallback};
pub use pool_utils::{
    call_slow_warn_threshold_ms, call_timeout_for_tool, hit_rate_pct_two_decimals,
    is_expected_long_running_tool, list_tools_cache_ttl_from_config,
};
pub use stats::{McpDiscoverCacheStatsSnapshot, McpToolsListCacheStatsSnapshot};
pub use task_handle::AbortOnDropJoinHandle;
pub use transport_error::{
    TransportErrorClass, classify_transport_error, should_retry_transport_error,
};
pub use wait_heartbeat::{
    WaitHeartbeatState, classify_wait_heartbeat, degraded_wait_warn_after_secs,
};
pub use wait_logger::{spawn_inflight_wait_logger, stop_wait_logger};
pub use xiuxian_mcp::{McpServerTransportConfig, OmniMcpClient, init_params_omni_server};
