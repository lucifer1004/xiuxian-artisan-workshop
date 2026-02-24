//! MCP pool stats payloads.

use serde::Serialize;

/// Snapshot of Rust-side `tools/list` cache behavior in MCP client pool.
#[derive(Debug, Clone, Serialize)]
pub struct McpToolsListCacheStatsSnapshot {
    /// Cache TTL in milliseconds.
    pub ttl_ms: u64,
    /// Total cache requests.
    pub requests_total: u64,
    /// Cache hit count.
    pub cache_hits: u64,
    /// Cache miss count.
    pub cache_misses: u64,
    /// Cache refresh count.
    pub cache_refreshes: u64,
    /// Hit ratio as percentage with two decimals.
    pub hit_rate_pct: f64,
}

/// Snapshot of discover call-cache behavior in MCP pool.
#[derive(Debug, Clone, Serialize)]
pub struct McpDiscoverCacheStatsSnapshot {
    /// Backend identifier.
    pub backend: String,
    /// Backend TTL in seconds.
    pub ttl_secs: u64,
    /// Total cache requests.
    pub requests_total: u64,
    /// Cache hit count.
    pub cache_hits: u64,
    /// Cache miss count.
    pub cache_misses: u64,
    /// Cache write count.
    pub cache_writes: u64,
    /// Hit ratio as percentage with two decimals.
    pub hit_rate_pct: f64,
}
