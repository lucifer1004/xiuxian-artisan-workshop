//! MCP pool retry orchestration helpers shared by runtime consumers.

use anyhow::{Result, anyhow};
use rmcp::model::{CallToolResult, ListToolsResult};

use crate::mcp::{classify_transport_error, should_retry_transport_error};

fn push_attempt_error(
    attempt_errors: &mut Vec<String>,
    client_index: usize,
    stage: &str,
    error_class: &str,
    error: &anyhow::Error,
) {
    attempt_errors.push(format!(
        "client_index={client_index},stage={stage},error_class={error_class},error={error}"
    ));
}

fn log_fallback_success(start_idx: usize, client_index: usize, previous_failures: usize) {
    tracing::info!(
        event = "mcp.pool.tools_list.fallback_client_used",
        start_index = start_idx,
        client_index,
        previous_failures,
        "mcp tools/list succeeded via fallback client"
    );
}

async fn retry_tools_list_after_reconnect<FList, FListFuture, FReconnect, FReconnectFuture>(
    start_idx: usize,
    client_index: usize,
    error: &anyhow::Error,
    attempt_errors: &mut Vec<String>,
    list_once: &mut FList,
    reconnect_once: &mut FReconnect,
) -> Option<ListToolsResult>
where
    FList: FnMut(usize) -> FListFuture,
    FListFuture: std::future::Future<Output = Result<ListToolsResult>>,
    FReconnect: FnMut(usize) -> FReconnectFuture,
    FReconnectFuture: std::future::Future<Output = Result<()>>,
{
    match reconnect_once(client_index).await {
        Ok(()) => match list_once(client_index).await {
            Ok(output) => Some(output),
            Err(retry_error) => {
                let retry_error_class = classify_transport_error(&retry_error);
                tracing::warn!(
                    event = "mcp.pool.call.failed_after_retry",
                    operation = "tools/list",
                    start_index = start_idx,
                    client_index,
                    error_class = retry_error_class.kind,
                    first_error = %error,
                    retry_error = %retry_error,
                    "mcp tools/list retry failed; attempting next pool client"
                );
                push_attempt_error(
                    attempt_errors,
                    client_index,
                    "retry",
                    retry_error_class.kind,
                    &retry_error,
                );
                None
            }
        },
        Err(reconnect_error) => {
            let reconnect_error_class = classify_transport_error(&reconnect_error);
            tracing::warn!(
                event = "mcp.pool.client.reconnect.failed",
                operation = "tools/list",
                start_index = start_idx,
                client_index,
                error_class = reconnect_error_class.kind,
                error = %reconnect_error,
                "mcp tools/list reconnect failed; attempting next pool client"
            );
            push_attempt_error(
                attempt_errors,
                client_index,
                "reconnect",
                reconnect_error_class.kind,
                &reconnect_error,
            );
            None
        }
    }
}

async fn handle_tools_list_attempt_error<FList, FListFuture, FReconnect, FReconnectFuture>(
    start_idx: usize,
    client_index: usize,
    error: &anyhow::Error,
    attempt_errors: &mut Vec<String>,
    list_once: &mut FList,
    reconnect_once: &mut FReconnect,
) -> Option<ListToolsResult>
where
    FList: FnMut(usize) -> FListFuture,
    FListFuture: std::future::Future<Output = Result<ListToolsResult>>,
    FReconnect: FnMut(usize) -> FReconnectFuture,
    FReconnectFuture: std::future::Future<Output = Result<()>>,
{
    let error_class = classify_transport_error(error);
    if error_class.retryable {
        tracing::warn!(
            event = "mcp.pool.call.retry.transport_error",
            operation = "tools/list",
            start_index = start_idx,
            client_index,
            error_class = error_class.kind,
            error = %error,
            "recoverable mcp tools/list transport error; attempting reconnect + retry"
        );
        return retry_tools_list_after_reconnect(
            start_idx,
            client_index,
            error,
            attempt_errors,
            list_once,
            reconnect_once,
        )
        .await;
    }

    tracing::warn!(
        event = "mcp.pool.call.failed",
        operation = "tools/list",
        start_index = start_idx,
        client_index,
        error_class = error_class.kind,
        error = %error,
        "mcp tools/list failed on client; attempting next pool client"
    );
    push_attempt_error(
        attempt_errors,
        client_index,
        "call",
        error_class.kind,
        error,
    );
    None
}

fn joined_attempt_errors(attempt_errors: &[String]) -> String {
    if attempt_errors.is_empty() {
        "no_attempts_recorded".to_string()
    } else {
        attempt_errors.join(" | ")
    }
}

/// Execute `tools/list` with round-robin fallback + reconnect retry.
///
/// `list_once` should attempt `tools/list` on a single client index.
/// `reconnect_once` should reconnect the given client index.
///
/// # Errors
/// Returns an error when all clients fail list execution, including retry paths.
pub async fn run_tools_list_with_fallback<FList, FListFuture, FReconnect, FReconnectFuture>(
    pool_size: usize,
    start_idx: usize,
    mut list_once: FList,
    mut reconnect_once: FReconnect,
) -> Result<ListToolsResult>
where
    FList: FnMut(usize) -> FListFuture,
    FListFuture: std::future::Future<Output = Result<ListToolsResult>>,
    FReconnect: FnMut(usize) -> FReconnectFuture,
    FReconnectFuture: std::future::Future<Output = Result<()>>,
{
    let mut attempt_errors: Vec<String> = Vec::with_capacity(pool_size);
    for offset in 0..pool_size {
        let client_index = (start_idx + offset) % pool_size;
        match list_once(client_index).await {
            Ok(output) => {
                if offset > 0 {
                    log_fallback_success(start_idx, client_index, offset);
                }
                return Ok(output);
            }
            Err(error) => {
                if let Some(output) = handle_tools_list_attempt_error(
                    start_idx,
                    client_index,
                    &error,
                    &mut attempt_errors,
                    &mut list_once,
                    &mut reconnect_once,
                )
                .await
                {
                    return Ok(output);
                }
            }
        }
    }

    let joined_errors = joined_attempt_errors(&attempt_errors);
    Err(anyhow!(
        "MCP tools/list failed on all clients (pool_size={pool_size}, start_index={start_idx}, attempts={joined_errors})"
    ))
}

/// Execute a single `tools/call` with reconnect-on-retryable transport error.
///
/// `call_once` should attempt the tool call one time.
/// `reconnect_once` should reconnect the selected client.
///
/// # Errors
/// Returns an error when the primary call and retry path both fail.
pub async fn run_tool_call_with_retry<FCall, FCallFuture, FReconnect, FReconnectFuture>(
    operation: &str,
    client_index: usize,
    timeout_secs: u64,
    mut call_once: FCall,
    mut reconnect_once: FReconnect,
) -> Result<CallToolResult>
where
    FCall: FnMut() -> FCallFuture,
    FCallFuture: std::future::Future<Output = Result<CallToolResult>>,
    FReconnect: FnMut() -> FReconnectFuture,
    FReconnectFuture: std::future::Future<Output = Result<()>>,
{
    let call_result = call_once().await;
    match call_result {
        Ok(output) => Ok(output),
        Err(error) if should_retry_transport_error(&error) => {
            tracing::warn!(
                event = "mcp.pool.call.retry.transport_error",
                operation = operation,
                client_index,
                timeout_secs,
                error = %error,
                "recoverable mcp tools/call transport error; attempting reconnect + retry"
            );
            reconnect_once().await?;
            let retry_result = call_once().await;
            match retry_result {
                Ok(output) => Ok(output),
                Err(retry_error) => {
                    tracing::error!(
                        event = "mcp.pool.call.failed_after_retry",
                        operation = operation,
                        client_index,
                        timeout_secs,
                        first_error = %error,
                        retry_error = %retry_error,
                        "mcp tools/call failed after reconnect retry"
                    );
                    Err(anyhow!(
                        "MCP tools/call failed after reconnect retry (client_index={client_index}, tool={operation}, first_error={error}, retry_error={retry_error})"
                    ))
                }
            }
        }
        Err(error) => {
            tracing::error!(
                event = "mcp.pool.call.failed",
                operation = operation,
                client_index,
                timeout_secs,
                error = %error,
                "mcp tools/call failed"
            );
            Err(error)
        }
    }
}
