use super::Agent;
use crate::{ToolClientPool, ToolDiscoverCacheStatsSnapshot, ToolListCacheStatsSnapshot};

impl Agent {
    /// Return Rust external tool client-pool `tools/list` cache snapshot when the pool is enabled.
    pub fn inspect_tool_list_cache_stats(&self) -> Option<ToolListCacheStatsSnapshot> {
        self.tool_runtime
            .as_ref()
            .map(ToolClientPool::tools_list_cache_stats_snapshot)
    }

    /// Return Rust external tool discover read-through cache stats when the pool is enabled.
    pub fn inspect_tool_discover_cache_stats(&self) -> Option<ToolDiscoverCacheStatsSnapshot> {
        self.tool_runtime
            .as_ref()
            .and_then(ToolClientPool::discover_cache_stats_snapshot)
    }
}
