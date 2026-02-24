use super::Agent;
use crate::mcp::{McpDiscoverCacheStatsSnapshot, McpToolsListCacheStatsSnapshot};

impl Agent {
    /// Return Rust MCP pool `tools/list` cache snapshot when MCP is enabled.
    pub fn inspect_mcp_tools_list_cache_stats(&self) -> Option<McpToolsListCacheStatsSnapshot> {
        self.mcp
            .as_ref()
            .map(super::super::mcp::McpClientPool::tools_list_cache_stats_snapshot)
    }

    /// Return Rust MCP discover read-through cache stats when enabled.
    pub fn inspect_mcp_discover_cache_stats(&self) -> Option<McpDiscoverCacheStatsSnapshot> {
        self.mcp
            .as_ref()
            .and_then(super::super::mcp::McpClientPool::discover_cache_stats_snapshot)
    }
}
