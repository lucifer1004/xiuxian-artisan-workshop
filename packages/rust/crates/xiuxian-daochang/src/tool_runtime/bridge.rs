use std::sync::Arc;

use anyhow::Result;

pub(crate) use xiuxian_llm::mcp::{
    DiscoverCacheConfig as ToolDiscoverCacheConfig,
    DiscoverReadThroughCache as ToolDiscoverReadThroughCache, McpClientPool as ToolClientPool,
    McpDiscoverCacheStatsSnapshot as ToolDiscoverCacheStatsSnapshot,
    McpPoolConnectConfig as ToolPoolConnectConfig,
    McpToolsListCacheStatsSnapshot as ToolListCacheStatsSnapshot,
};

pub(crate) async fn connect_tool_pool_backend(
    url: &str,
    config: ToolPoolConnectConfig,
    discover_cache: Option<Arc<ToolDiscoverReadThroughCache>>,
) -> Result<ToolClientPool> {
    xiuxian_llm::mcp::connect_pool(url, config, discover_cache).await
}
