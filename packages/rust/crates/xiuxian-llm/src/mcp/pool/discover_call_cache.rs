//! Discover-call cache operations and stats for MCP pool.

use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::mcp::hit_rate_pct_two_decimals;
use rmcp::model::CallToolResult;

use super::McpClientPool;
use crate::mcp::McpDiscoverCacheStatsSnapshot;

impl McpClientPool {
    /// Return discover cache stats when discover read-through cache is enabled.
    pub fn discover_cache_stats_snapshot(&self) -> Option<McpDiscoverCacheStatsSnapshot> {
        let cache = self.discover_cache.as_ref()?;
        let runtime = cache.runtime_info();
        let hits = self.discover_cache_hits.load(Ordering::Relaxed);
        let misses = self.discover_cache_misses.load(Ordering::Relaxed);
        let writes = self.discover_cache_writes.load(Ordering::Relaxed);
        let requests = hits.saturating_add(misses);
        let hit_rate_pct = hit_rate_pct_two_decimals(hits, requests);
        Some(McpDiscoverCacheStatsSnapshot {
            backend: runtime.backend.to_string(),
            ttl_secs: runtime.ttl_secs,
            requests_total: requests,
            cache_hits: hits,
            cache_misses: misses,
            cache_writes: writes,
            hit_rate_pct,
        })
    }

    pub(super) async fn get_cached_discover_call(&self, cache_key: &str) -> Option<CallToolResult> {
        let cache = self.discover_cache.as_ref()?;
        match cache.get(cache_key).await {
            Ok(Some(cached)) => {
                self.discover_cache_hits.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(
                    event = "mcp.pool.discover_cache.hit",
                    cache_key,
                    "discover call served from cache"
                );
                self.maybe_log_discover_cache_stats();
                Some(cached)
            }
            Ok(None) => {
                self.discover_cache_misses.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(
                    event = "mcp.pool.discover_cache.miss",
                    cache_key,
                    "discover call cache miss"
                );
                self.maybe_log_discover_cache_stats();
                None
            }
            Err(error) => {
                self.discover_cache_misses.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    event = "mcp.pool.discover_cache.get_failed",
                    cache_key,
                    error = %error,
                    "discover call cache read failed; continuing without cache"
                );
                self.maybe_log_discover_cache_stats();
                None
            }
        }
    }

    pub(super) async fn store_discover_call_cache(&self, cache_key: &str, output: &CallToolResult) {
        if matches!(output.is_error, Some(true)) {
            return;
        }
        let Some(cache) = self.discover_cache.as_ref() else {
            return;
        };
        match cache.set(cache_key, output).await {
            Ok(()) => {
                self.discover_cache_writes.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(
                    event = "mcp.pool.discover_cache.write",
                    cache_key,
                    "discover call cached"
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "mcp.pool.discover_cache.write_failed",
                    cache_key,
                    error = %error,
                    "discover call cache write failed"
                );
            }
        }
        self.maybe_log_discover_cache_stats();
    }

    pub(super) fn maybe_log_discover_cache_stats(&self) {
        if self.discover_cache.is_none() {
            return;
        }
        let Ok(mut last_log_at) = self.discover_cache_last_log_at.try_lock() else {
            return;
        };
        if last_log_at.elapsed() < self.discover_cache_stats_log_interval {
            return;
        }
        *last_log_at = Instant::now();

        let Some(snapshot) = self.discover_cache_stats_snapshot() else {
            return;
        };

        tracing::info!(
            event = "mcp.pool.discover_cache.stats",
            backend = %snapshot.backend,
            ttl_secs = snapshot.ttl_secs,
            requests_total = snapshot.requests_total,
            cache_hits = snapshot.cache_hits,
            cache_misses = snapshot.cache_misses,
            cache_writes = snapshot.cache_writes,
            hit_rate_pct = snapshot.hit_rate_pct,
            "mcp discover cache stats"
        );
    }
}
