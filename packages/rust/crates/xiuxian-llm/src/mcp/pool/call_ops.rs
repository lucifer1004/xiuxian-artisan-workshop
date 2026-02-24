//! MCP pool call operations and retry orchestration.

use std::sync::atomic::Ordering;

use crate::mcp::{call_timeout_for_tool, call_tool_once, run_tool_call_with_retry};
use anyhow::Result;
use rmcp::model::CallToolResult;

use super::McpClientPool;

impl McpClientPool {
    /// Call a tool; round-robin picks a client so concurrent calls use different clients.
    ///
    /// # Errors
    /// Returns an error when both primary and retry `tools/call` paths fail, or reconnect fails.
    pub async fn call_tool(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<CallToolResult> {
        let discover_cache_key = self
            .discover_cache
            .as_ref()
            .and_then(|cache| cache.build_cache_key(name.as_str(), arguments.as_ref()));
        if let Some(cache_key) = discover_cache_key.as_deref()
            && let Some(cached) = self.get_cached_discover_call(cache_key).await
        {
            return Ok(cached);
        }

        let idx = self.next.fetch_add(1, Ordering::Relaxed) % self.pool_size;
        let call_timeout = call_timeout_for_tool(name.as_str(), self.tool_timeout);
        let operation = format!("tools/call:{name}");
        let server_url = self.server_url.clone();
        let base_name = name.clone();
        let base_arguments = arguments.clone();
        let call_result = run_tool_call_with_retry(
            &operation,
            idx,
            call_timeout.as_secs(),
            || {
                let name = base_name.clone();
                let arguments = base_arguments.clone();
                let server_url = server_url.clone();
                async move {
                    match self.client(idx).await {
                        Ok(client) => {
                            call_tool_once(client, server_url, idx, name, arguments, call_timeout)
                                .await
                        }
                        Err(error) => Err(error),
                    }
                }
            },
            || async {
                self.reconnect_client(idx, "tools/call transport error")
                    .await
            },
        )
        .await;

        if let Some(cache_key) = discover_cache_key.as_deref()
            && let Ok(output) = call_result.as_ref()
        {
            self.store_discover_call_cache(cache_key, output).await;
        }

        call_result
    }
}
