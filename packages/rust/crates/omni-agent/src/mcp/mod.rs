//! MCP integration namespace for omni-agent.

mod discover_cache;

use anyhow::Result;
pub use xiuxian_llm::mcp::{
    McpClientPool, McpDiscoverCacheStatsSnapshot, McpPoolConnectConfig,
    McpToolsListCacheStatsSnapshot,
};

/// Build pool from URL with runtime-resolved discover-cache wiring.
///
/// # Errors
/// Returns an error when discover-cache initialization or MCP pool bootstrap fails.
pub async fn connect_pool(url: &str, config: McpPoolConnectConfig) -> Result<McpClientPool> {
    let discover_cache = match discover_cache::discover_cache_from_runtime() {
        Ok(cache) => cache,
        Err(error) => {
            tracing::warn!(
                event = "mcp.pool.discover_cache.init_failed",
                error = %error,
                "discover read-through cache init failed; continuing without cache"
            );
            None
        }
    };
    if let Some(cache) = discover_cache.as_ref() {
        let runtime = cache.runtime_info();
        tracing::info!(
            event = "mcp.pool.discover_cache.enabled",
            backend = runtime.backend,
            ttl_secs = runtime.ttl_secs,
            "discover read-through cache enabled"
        );
    }
    xiuxian_llm::mcp::connect_pool(url, config, discover_cache).await
}
