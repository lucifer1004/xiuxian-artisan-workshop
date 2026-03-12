//! MCP client pool for concurrent tool calls.
//!
//! Multiple Telegram groups (or gateway requests) can call tools concurrently.
//! A single MCP client uses a Mutex, serializing all calls. This pool holds
//! N clients and uses round-robin so up to N tool calls run in parallel.

mod bootstrap;
mod call_ops;
mod discover_call_cache;
mod lifecycle;
mod list_ops;
mod tools_list_cache;

use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::time::{Duration, Instant};

use super::discover_cache::DiscoverReadThroughCache;
use anyhow::Result;
use rmcp::model::ListToolsResult;
use tokio::sync::{Mutex, RwLock};
pub use xiuxian_llm::mcp::McpPoolConnectConfig;
use xiuxian_llm::mcp::OmniMcpClient;

const DEFAULT_LIST_TOOLS_CACHE_STATS_LOG_INTERVAL_SECS: u64 = 60;
const DEFAULT_DISCOVER_CACHE_STATS_LOG_INTERVAL_SECS: u64 = 60;

#[derive(Clone)]
struct ListToolsCacheEntry {
    value: ListToolsResult,
    cached_at: Instant,
}

/// Pool of MCP clients for concurrent tool calls.
pub struct McpClientPool {
    server_url: String,
    connect_config: McpPoolConnectConfig,
    clients: RwLock<Vec<std::sync::Arc<OmniMcpClient>>>,
    reconnect_locks: Vec<Mutex<()>>,
    pool_size: usize,
    next: AtomicUsize,
    tool_timeout: Duration,
    list_tools_cache: RwLock<Option<ListToolsCacheEntry>>,
    list_tools_cache_lock: Mutex<()>,
    list_tools_cache_ttl: Duration,
    list_tools_cache_hits: AtomicU64,
    list_tools_cache_misses: AtomicU64,
    list_tools_cache_refreshes: AtomicU64,
    list_tools_cache_last_log_at: Mutex<Instant>,
    list_tools_cache_stats_log_interval: Duration,
    discover_cache: Option<std::sync::Arc<DiscoverReadThroughCache>>,
    discover_cache_hits: AtomicU64,
    discover_cache_misses: AtomicU64,
    discover_cache_writes: AtomicU64,
    discover_cache_last_log_at: Mutex<Instant>,
    discover_cache_stats_log_interval: Duration,
}

/// Build pool from URL with explicit connect configuration.
///
/// # Errors
/// Returns an error when MCP pool connection/bootstrap fails.
pub async fn connect_pool(url: &str, config: McpPoolConnectConfig) -> Result<McpClientPool> {
    McpClientPool::connect(url, config).await
}
