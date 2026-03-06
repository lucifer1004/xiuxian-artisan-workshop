//! MCP pool bootstrap and initial connection setup.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::time::{Duration, Instant};

use crate::mcp::{connect_pool_clients_with_retry, list_tools_cache_ttl_from_config};
use anyhow::Result;
use tokio::sync::{Mutex, RwLock};

use super::McpClientPool;
use crate::mcp::DiscoverReadThroughCache;

impl McpClientPool {
    /// Connect to MCP server and create a pool of clients.
    ///
    /// # Errors
    /// Returns an error when client handshake/retry bootstrap fails or pool initialization cannot complete.
    pub async fn connect(
        url: &str,
        config: super::super::McpPoolConnectConfig,
        discover_cache: Option<Arc<DiscoverReadThroughCache>>,
    ) -> Result<Self> {
        let clients = connect_pool_clients_with_retry(url, config).await?;
        let cache_stats_log_interval =
            Duration::from_secs(super::DEFAULT_LIST_TOOLS_CACHE_STATS_LOG_INTERVAL_SECS);
        let initial_cache_stats_log_at = Instant::now()
            .checked_sub(cache_stats_log_interval)
            .unwrap_or_else(Instant::now);
        if let Some(cache) = discover_cache.as_ref() {
            let runtime = cache.runtime_info();
            tracing::info!(
                event = "mcp.pool.discover_cache.enabled",
                backend = runtime.backend,
                ttl_secs = runtime.ttl_secs,
                "discover read-through cache enabled"
            );
        }
        let discover_cache_stats_log_interval =
            Duration::from_secs(super::DEFAULT_DISCOVER_CACHE_STATS_LOG_INTERVAL_SECS);
        let initial_discover_cache_stats_log_at = Instant::now()
            .checked_sub(discover_cache_stats_log_interval)
            .unwrap_or_else(Instant::now);
        Ok(Self {
            server_url: url.to_string(),
            connect_config: config,
            clients: RwLock::new(clients),
            reconnect_locks: (0..config.pool_size).map(|_| Mutex::new(())).collect(),
            pool_size: config.pool_size,
            next: AtomicUsize::new(0),
            tool_timeout: Duration::from_secs(config.tool_timeout_secs.max(1)),
            list_tools_cache: RwLock::new(None),
            list_tools_cache_lock: Mutex::new(()),
            list_tools_cache_ttl: list_tools_cache_ttl_from_config(config.list_tools_cache_ttl_ms),
            list_tools_cache_hits: AtomicU64::new(0),
            list_tools_cache_misses: AtomicU64::new(0),
            list_tools_cache_refreshes: AtomicU64::new(0),
            list_tools_cache_last_log_at: Mutex::new(initial_cache_stats_log_at),
            list_tools_cache_stats_log_interval: cache_stats_log_interval,
            observed_tool_list_changed_epoch: AtomicU64::new(0),
            discover_cache,
            discover_cache_hits: AtomicU64::new(0),
            discover_cache_misses: AtomicU64::new(0),
            discover_cache_writes: AtomicU64::new(0),
            discover_cache_last_log_at: Mutex::new(initial_discover_cache_stats_log_at),
            discover_cache_stats_log_interval,
        })
    }
}
