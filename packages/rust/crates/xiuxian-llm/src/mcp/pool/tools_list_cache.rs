//! `tools/list` cache operations and stats for MCP pool.

use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::mcp::hit_rate_pct_two_decimals;
use rmcp::model::ListToolsResult;

use super::{ListToolsCacheEntry, McpClientPool};
use crate::mcp::McpToolsListCacheStatsSnapshot;

impl McpClientPool {
    pub(super) async fn get_cached_list_tools(&self) -> Option<ListToolsResult> {
        let cache = self.list_tools_cache.read().await;
        let entry = cache.as_ref()?;
        if entry.cached_at.elapsed() <= self.list_tools_cache_ttl {
            return Some(entry.value.clone());
        }
        None
    }

    pub(super) async fn update_list_tools_cache(&self, fresh: &ListToolsResult) {
        let mut cache = self.list_tools_cache.write().await;
        *cache = Some(ListToolsCacheEntry {
            value: fresh.clone(),
            cached_at: Instant::now(),
        });
    }

    pub(super) async fn invalidate_list_tools_cache(&self) {
        let mut cache = self.list_tools_cache.write().await;
        *cache = None;
    }

    pub(super) fn record_list_tools_cache_hit(&self) {
        self.list_tools_cache_hits.fetch_add(1, Ordering::Relaxed);
        self.maybe_log_list_tools_cache_stats();
    }

    pub(super) fn record_list_tools_cache_miss(&self) {
        self.list_tools_cache_misses.fetch_add(1, Ordering::Relaxed);
        self.maybe_log_list_tools_cache_stats();
    }

    pub(super) fn record_list_tools_cache_refresh(&self) {
        self.list_tools_cache_refreshes
            .fetch_add(1, Ordering::Relaxed);
        self.maybe_log_list_tools_cache_stats();
    }

    pub(super) fn maybe_log_list_tools_cache_stats(&self) {
        let Ok(mut last_log_at) = self.list_tools_cache_last_log_at.try_lock() else {
            return;
        };
        if last_log_at.elapsed() < self.list_tools_cache_stats_log_interval {
            return;
        }
        *last_log_at = Instant::now();

        let snapshot = self.tools_list_cache_stats_snapshot();

        tracing::info!(
            event = "mcp.pool.tools_list.cache.stats",
            requests_total = snapshot.requests_total,
            cache_hits = snapshot.cache_hits,
            cache_misses = snapshot.cache_misses,
            cache_refreshes = snapshot.cache_refreshes,
            hit_rate_pct = snapshot.hit_rate_pct,
            ttl_ms = snapshot.ttl_ms,
            "mcp tools/list cache stats"
        );
    }

    /// Return a cheap point-in-time snapshot of `tools/list` cache behavior.
    pub fn tools_list_cache_stats_snapshot(&self) -> McpToolsListCacheStatsSnapshot {
        let hits = self.list_tools_cache_hits.load(Ordering::Relaxed);
        let misses = self.list_tools_cache_misses.load(Ordering::Relaxed);
        let refreshes = self.list_tools_cache_refreshes.load(Ordering::Relaxed);
        let requests = hits.saturating_add(misses);
        let hit_rate_pct = hit_rate_pct_two_decimals(hits, requests);
        McpToolsListCacheStatsSnapshot {
            ttl_ms: u64::try_from(self.list_tools_cache_ttl.as_millis()).unwrap_or(u64::MAX),
            requests_total: requests,
            cache_hits: hits,
            cache_misses: misses,
            cache_refreshes: refreshes,
            hit_rate_pct,
        }
    }
}
