//! Low-level MCP pool call/list execution paths.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use rmcp::model::{CallToolResult, ListToolsResult, PaginatedRequestParams};

use crate::mcp::{
    AbortOnDropJoinHandle, OmniMcpClient, call_slow_warn_threshold_ms, spawn_inflight_wait_logger,
    stop_wait_logger,
};

/// Execute one `tools/list` call with hard timeout guard and wait logging.
///
/// # Errors
/// Returns an error when MCP request fails, worker task join fails, or timeout is reached.
pub async fn list_tools_once(
    client: Arc<OmniMcpClient>,
    server_url: String,
    client_index: usize,
    timeout: Duration,
    params: Option<PaginatedRequestParams>,
) -> Result<ListToolsResult> {
    let started = std::time::Instant::now();
    let (wait_logger, wait_logger_stop) =
        spawn_inflight_wait_logger("tools/list".to_string(), server_url, client_index, timeout);
    let mut request_task =
        AbortOnDropJoinHandle::new(tokio::spawn(async move { client.list_tools(params).await }));
    let result = request_task.timeout(timeout).await;
    stop_wait_logger(wait_logger, wait_logger_stop).await;
    match result {
        Ok(Ok(Ok(output))) => {
            request_task.disarm();
            let elapsed_ms = started.elapsed().as_millis();
            let slow_warn_threshold_ms = call_slow_warn_threshold_ms("tools/list", timeout);
            if elapsed_ms >= slow_warn_threshold_ms {
                tracing::warn!(
                    event = "mcp.pool.call.slow",
                    operation = "tools/list",
                    client_index,
                    elapsed_ms,
                    slow_warn_threshold_ms,
                    timeout_secs = timeout.as_secs(),
                    "mcp tools/list completed slowly"
                );
            }
            Ok(output)
        }
        Ok(Ok(Err(error))) => {
            request_task.disarm();
            Err(error)
        }
        Ok(Err(join_error)) => {
            request_task.disarm();
            Err(anyhow!(
                "MCP tools/list worker task join failed (client_index={client_index}, error={join_error})"
            ))
        }
        Err(_) => {
            request_task.abort();
            request_task.disarm();
            tracing::warn!(
                event = "mcp.pool.call.timeout.hard",
                operation = "tools/list",
                client_index,
                timeout_secs = timeout.as_secs(),
                "mcp tools/list hard timeout reached; worker task aborted"
            );
            Err(anyhow!(
                "MCP tools/list timed out after {}s (client_index={})",
                timeout.as_secs(),
                client_index
            ))
        }
    }
}

/// Execute one `tools/call` request with hard timeout guard and wait logging.
///
/// # Errors
/// Returns an error when MCP request fails, worker task join fails, or timeout is reached.
pub async fn call_tool_once(
    client: Arc<OmniMcpClient>,
    server_url: String,
    client_index: usize,
    name: String,
    arguments: Option<serde_json::Value>,
    timeout: Duration,
) -> Result<CallToolResult> {
    let started = std::time::Instant::now();
    let operation = format!("tools/call:{name}");
    let (wait_logger, wait_logger_stop) =
        spawn_inflight_wait_logger(operation.clone(), server_url, client_index, timeout);
    let mut request_task = AbortOnDropJoinHandle::new(tokio::spawn(async move {
        client.call_tool(name, arguments).await
    }));
    let result = request_task.timeout(timeout).await;
    stop_wait_logger(wait_logger, wait_logger_stop).await;
    match result {
        Ok(Ok(Ok(output))) => {
            request_task.disarm();
            let elapsed_ms = started.elapsed().as_millis();
            let slow_warn_threshold_ms = call_slow_warn_threshold_ms(&operation, timeout);
            if elapsed_ms >= slow_warn_threshold_ms {
                tracing::warn!(
                    event = "mcp.pool.call.slow",
                    operation = %operation,
                    client_index,
                    elapsed_ms,
                    slow_warn_threshold_ms,
                    timeout_secs = timeout.as_secs(),
                    "mcp tools/call completed slowly"
                );
            }
            Ok(output)
        }
        Ok(Ok(Err(error))) => {
            request_task.disarm();
            Err(error)
        }
        Ok(Err(join_error)) => {
            request_task.disarm();
            Err(anyhow!(
                "MCP tools/call worker task join failed (client_index={client_index}, tool={operation}, error={join_error})"
            ))
        }
        Err(_) => {
            request_task.abort();
            request_task.disarm();
            tracing::warn!(
                event = "mcp.pool.call.timeout.hard",
                operation = %operation,
                client_index,
                timeout_secs = timeout.as_secs(),
                "mcp tools/call hard timeout reached; worker task aborted"
            );
            Err(anyhow!(
                "MCP tools/call timed out after {}s (client_index={}, tool={})",
                timeout.as_secs(),
                client_index,
                operation
            ))
        }
    }
}
