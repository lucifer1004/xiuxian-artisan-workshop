//! MCP pool connection configuration.

const DEFAULT_MCP_POOL_SIZE: usize = 4;
const DEFAULT_MCP_HANDSHAKE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MCP_CONNECT_RETRIES: u32 = 3;
const DEFAULT_MCP_CONNECT_RETRY_BACKOFF_MS: u64 = 1_000;
const DEFAULT_MCP_TOOL_TIMEOUT_SECS: u64 = 180;
const DEFAULT_MCP_LIST_TOOLS_CACHE_TTL_MS: u64 = 1_000;

/// Connection settings for MCP client pools.
#[derive(Debug, Clone, Copy)]
pub struct McpPoolConnectConfig {
    /// Number of MCP clients in pool.
    pub pool_size: usize,
    /// Per-client handshake timeout in seconds.
    pub handshake_timeout_secs: u64,
    /// Retry attempts for connection/bootstrap.
    pub connect_retries: u32,
    /// Initial reconnect backoff in milliseconds.
    pub connect_retry_backoff_ms: u64,
    /// Tool call timeout in seconds.
    pub tool_timeout_secs: u64,
    /// Local `tools/list` cache TTL in milliseconds.
    pub list_tools_cache_ttl_ms: u64,
}

impl Default for McpPoolConnectConfig {
    fn default() -> Self {
        Self {
            pool_size: DEFAULT_MCP_POOL_SIZE,
            handshake_timeout_secs: DEFAULT_MCP_HANDSHAKE_TIMEOUT_SECS,
            connect_retries: DEFAULT_MCP_CONNECT_RETRIES,
            connect_retry_backoff_ms: DEFAULT_MCP_CONNECT_RETRY_BACKOFF_MS,
            tool_timeout_secs: DEFAULT_MCP_TOOL_TIMEOUT_SECS,
            list_tools_cache_ttl_ms: DEFAULT_MCP_LIST_TOOLS_CACHE_TTL_MS,
        }
    }
}
