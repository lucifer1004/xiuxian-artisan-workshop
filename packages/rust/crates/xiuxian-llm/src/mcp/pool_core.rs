//! MCP pool core connect/reconnect helpers shared by runtime consumers.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::task::JoinSet;

use crate::mcp::{McpPoolConnectConfig, connect_one_client_with_retry};
use xiuxian_mcp::OmniMcpClient;

/// Connect all MCP clients for a pool with bounded retries.
///
/// # Errors
/// Returns an error when `pool_size` is zero or any client connect attempt fails.
pub async fn connect_pool_clients_with_retry(
    url: &str,
    config: McpPoolConnectConfig,
) -> Result<Vec<Arc<OmniMcpClient>>> {
    if config.pool_size == 0 {
        return Err(anyhow!("MCP pool_size must be greater than 0"));
    }

    let retries = config.connect_retries.max(1);
    let mut clients: Vec<Arc<OmniMcpClient>> = Vec::with_capacity(config.pool_size);
    let first_client = connect_one_client_with_retry(url, config, retries, 0).await?;
    clients.push(Arc::new(first_client));

    if config.pool_size > 1 {
        let mut connect_tasks = JoinSet::new();
        for client_index in 1..config.pool_size {
            let url = url.to_string();
            connect_tasks.spawn(async move {
                connect_one_client_with_retry(&url, config, retries, client_index)
                    .await
                    .map(Arc::new)
            });
        }

        while let Some(task_result) = connect_tasks.join_next().await {
            match task_result {
                Ok(Ok(client)) => clients.push(client),
                Ok(Err(error)) => {
                    connect_tasks.abort_all();
                    return Err(error);
                }
                Err(join_error) => {
                    connect_tasks.abort_all();
                    return Err(anyhow!("MCP connect task join failed: {join_error}"));
                }
            }
        }
    }

    Ok(clients)
}

/// Reconnect one MCP client slot with bounded retries.
///
/// # Errors
/// Returns an error when reconnect attempt fails.
pub async fn reconnect_pool_client_with_retry(
    url: &str,
    config: McpPoolConnectConfig,
    client_index: usize,
) -> Result<Arc<OmniMcpClient>> {
    let retries = config.connect_retries.max(1);
    connect_one_client_with_retry(url, config, retries, client_index)
        .await
        .map(Arc::new)
}
