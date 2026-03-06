//! MCP tool-call execution helpers.
//!
//! These helpers keep timeout/error classification and response decoding in one
//! place so callers can focus on orchestration.

use std::future::Future;
use std::time::Duration;

use anyhow::Error;
use rmcp::model::{CallToolResult, RawContent};

use crate::{OmniMcpClient, is_timeout_error_message};

/// Decoded MCP tool output shape used by upper layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolCallOutput {
    /// Text payload concatenated from all MCP text content items.
    pub text: String,
    /// Whether MCP marks the call result as error.
    pub is_error: bool,
}

/// Result of one MCP tool execution under timeout control.
#[derive(Debug)]
pub enum McpToolCallExecution {
    /// Call completed and output was decoded.
    Completed(McpToolCallOutput),
    /// Timeout reached (either local timeout or timeout-like MCP error).
    Timeout {
        /// Optional timeout detail from transport error text.
        detail: Option<String>,
    },
    /// Non-timeout MCP transport failure.
    TransportError(Error),
}

/// Call one MCP tool with hard timeout and timeout-class degradation semantics.
pub async fn execute_call_with_timeout<F, Fut>(
    operation: F,
    timeout_secs: u64,
) -> McpToolCallExecution
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<CallToolResult>>,
{
    let timeout_secs = timeout_secs.max(1);
    let call_result = tokio::time::timeout(Duration::from_secs(timeout_secs), operation()).await;

    match call_result {
        Err(_) => McpToolCallExecution::Timeout { detail: None },
        Ok(Err(error)) => {
            let message = format!("{error:#}");
            if is_timeout_error_message(&message) {
                McpToolCallExecution::Timeout {
                    detail: Some(message),
                }
            } else {
                McpToolCallExecution::TransportError(error)
            }
        }
        Ok(Ok(result)) => McpToolCallExecution::Completed(decode_call_tool_result(&result)),
    }
}

/// Call one `OmniMcpClient` tool with hard timeout and timeout-class
/// degradation semantics.
pub async fn call_tool_with_timeout(
    client: &OmniMcpClient,
    name: &str,
    arguments: Option<serde_json::Value>,
    timeout_secs: u64,
) -> McpToolCallExecution {
    execute_call_with_timeout(
        || client.call_tool(name.to_string(), arguments),
        timeout_secs,
    )
    .await
}

/// Decode `CallToolResult` to plain text + error flag.
#[must_use]
pub fn decode_call_tool_result(result: &CallToolResult) -> McpToolCallOutput {
    let text: String = result
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect();

    McpToolCallOutput {
        text,
        is_error: result.is_error.unwrap_or(false),
    }
}
