//! MCP pool list operations and retry orchestration.

use std::sync::atomic::Ordering;

use crate::mcp::{list_tools_once, run_tools_list_with_fallback};
use anyhow::Result;
use rmcp::model::{ListToolsResult, PaginatedRequestParams};

use super::McpClientPool;

impl McpClientPool {
    /// List tools (uses first client).
    ///
    /// # Errors
    /// Returns an error when all clients fail `tools/list` execution, including reconnect retries.
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult> {
        if params.is_some() {
            return self.list_tools_uncached(params).await;
        }

        self.sync_list_tools_cache_with_server_notifications().await;
        if let Some(cached) = self.get_cached_list_tools().await {
            self.record_list_tools_cache_hit();
            tracing::debug!(
                event = "mcp.pool.tools_list.cache_hit",
                ttl_ms = self.list_tools_cache_ttl.as_millis(),
                "mcp tools/list served from cache"
            );
            return Ok(cached);
        }

        let _cache_guard = self.list_tools_cache_lock.lock().await;
        self.sync_list_tools_cache_with_server_notifications().await;
        if let Some(cached) = self.get_cached_list_tools().await {
            self.record_list_tools_cache_hit();
            tracing::debug!(
                event = "mcp.pool.tools_list.cache_hit_after_wait",
                ttl_ms = self.list_tools_cache_ttl.as_millis(),
                "mcp tools/list served from cache after waiting for in-flight refresh"
            );
            return Ok(cached);
        }

        self.record_list_tools_cache_miss();
        let fresh = self.list_tools_uncached(None).await?;
        self.update_list_tools_cache(&fresh).await;
        self.record_list_tools_cache_refresh();
        Ok(fresh)
    }

    async fn list_tools_uncached(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult> {
        let start_idx = self.next.fetch_add(1, Ordering::Relaxed) % self.pool_size;
        run_tools_list_with_fallback(
            self.pool_size,
            start_idx,
            |client_index| {
                let params = params.clone();
                async move {
                    match self.client(client_index).await {
                        Ok(client) => {
                            list_tools_once(
                                client,
                                self.server_url.clone(),
                                client_index,
                                self.tool_timeout,
                                params,
                            )
                            .await
                        }
                        Err(error) => Err(error),
                    }
                }
            },
            |client_index| async move {
                self.reconnect_client(client_index, "tools/list transport error")
                    .await
            },
        )
        .await
    }
}
