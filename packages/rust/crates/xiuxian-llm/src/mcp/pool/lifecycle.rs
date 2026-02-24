//! MCP pool client lifecycle operations.

use std::sync::Arc;

use crate::mcp::reconnect_pool_client_with_retry;
use anyhow::{Result, anyhow};

use super::McpClientPool;

impl McpClientPool {
    pub(super) async fn client(
        &self,
        client_index: usize,
    ) -> Result<Arc<crate::mcp::OmniMcpClient>> {
        let clients = self.clients.read().await;
        clients
            .get(client_index)
            .cloned()
            .ok_or_else(|| anyhow!("MCP pool client index out of bounds: {client_index}"))
    }

    pub(super) async fn reconnect_client(&self, client_index: usize, reason: &str) -> Result<()> {
        let reconnect_lock = self
            .reconnect_locks
            .get(client_index)
            .ok_or_else(|| anyhow!("MCP reconnect lock index out of bounds: {client_index}"))?;
        let _guard = reconnect_lock.lock().await;
        let retries = self.connect_config.connect_retries.max(1);
        let new_client =
            reconnect_pool_client_with_retry(&self.server_url, self.connect_config, client_index)
                .await?;
        let mut clients = self.clients.write().await;
        if client_index >= clients.len() {
            return Err(anyhow!(
                "MCP reconnect client index out of bounds: {client_index}"
            ));
        }
        clients[client_index] = new_client;
        drop(clients);
        self.invalidate_list_tools_cache().await;
        tracing::info!(
            event = "mcp.pool.client.reconnected",
            url = %self.server_url,
            client_index,
            reason,
            retries,
            "mcp pool client reconnected"
        );
        Ok(())
    }
}
