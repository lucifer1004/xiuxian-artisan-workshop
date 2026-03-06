use std::time::Duration;

use anyhow::Result;

use super::super::Agent;
use super::diagnostics::{
    log_tool_dispatch_error, log_tool_dispatch_error_with_detail, log_tool_dispatch_success,
    log_tool_dispatch_timeout, log_tool_dispatch_timeout_with_detail, tool_timeout_error_output,
};
use super::tool_types::ToolCallOutput;

impl Agent {
    /// Primary tool dispatcher: Native first, then Zhenfa bridge, then MCP.
    pub(in crate::agent) async fn call_mcp_tool_with_diagnostics(
        &self,
        session_id: Option<&str>,
        tool_call_id: Option<&str>,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallOutput> {
        if let Some(output) = self
            .call_native_tool_with_diagnostics(session_id, tool_call_id, name, arguments.as_ref())
            .await
        {
            return Ok(output);
        }

        if let Some(output) = self
            .call_zhenfa_tool_with_diagnostics(session_id, tool_call_id, name, arguments.as_ref())
            .await
        {
            return Ok(output);
        }

        self.call_external_mcp_tool_with_diagnostics(session_id, tool_call_id, name, arguments)
            .await
    }

    async fn call_native_tool_with_diagnostics(
        &self,
        session_id: Option<&str>,
        tool_call_id: Option<&str>,
        name: &str,
        arguments: Option<&serde_json::Value>,
    ) -> Option<ToolCallOutput> {
        let native_tool = self.native_tools.get(name)?;
        let timeout_secs = self.config.mcp_tool_timeout_secs.max(1);
        let context = super::super::native_tools::registry::NativeToolCallContext {
            session_id: session_id.map(ToString::to_string),
            tool_call_id: tool_call_id.map(ToString::to_string),
        };
        let call_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            native_tool.call(arguments.cloned(), &context),
        )
        .await;

        Some(if let Ok(result) = call_result {
            match result {
                Ok(text) => {
                    log_tool_dispatch_success("native", name, session_id);
                    ToolCallOutput::success(text, tool_call_id)
                }
                Err(error) => {
                    log_tool_dispatch_error_with_detail(
                        "native",
                        name,
                        session_id,
                        "error",
                        &error,
                        "tool dispatch completed with native tool error",
                    );
                    ToolCallOutput::error(format!("Native tool error: {error}"), tool_call_id)
                }
            }
        } else {
            log_tool_dispatch_timeout("native", name, session_id, timeout_secs);
            tool_timeout_error_output("native", name, timeout_secs, tool_call_id)
        })
    }

    async fn call_zhenfa_tool_with_diagnostics(
        &self,
        session_id: Option<&str>,
        tool_call_id: Option<&str>,
        name: &str,
        arguments: Option<&serde_json::Value>,
    ) -> Option<ToolCallOutput> {
        let zhenfa_tools = self.zhenfa_tools.as_ref()?;
        let timeout_secs = self.config.mcp_tool_timeout_secs.max(1);
        if !zhenfa_tools.handles_tool(name) {
            return None;
        }
        let call_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            zhenfa_tools.call_tool(session_id, name, arguments.cloned()),
        )
        .await;

        Some(if let Ok(result) = call_result {
            match result {
                Ok(text) => {
                    log_tool_dispatch_success("zhenfa", name, session_id);
                    ToolCallOutput::success(text, tool_call_id)
                }
                Err(error) => {
                    log_tool_dispatch_error_with_detail(
                        "zhenfa",
                        name,
                        session_id,
                        "error",
                        &error,
                        "tool dispatch completed with zhenfa tool error",
                    );
                    ToolCallOutput::error(format!("Zhenfa tool error: {error}"), tool_call_id)
                }
            }
        } else {
            log_tool_dispatch_timeout("zhenfa", name, session_id, timeout_secs);
            tool_timeout_error_output("zhenfa", name, timeout_secs, tool_call_id)
        })
    }

    async fn call_external_mcp_tool_with_diagnostics(
        &self,
        session_id: Option<&str>,
        tool_call_id: Option<&str>,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallOutput> {
        let timeout_secs = self.config.mcp_tool_timeout_secs.max(1);
        let Some(ref mcp) = self.mcp else {
            return Err(anyhow::anyhow!(
                "no native tool, zhenfa tool, or MCP client found for `{name}`"
            ));
        };

        match xiuxian_mcp::execute_call_with_timeout(
            || mcp.call_tool(name.to_string(), arguments),
            timeout_secs,
        )
        .await
        {
            xiuxian_mcp::McpToolCallExecution::Completed(payload) => {
                if payload.is_error {
                    log_tool_dispatch_error(
                        "mcp",
                        name,
                        session_id,
                        "tool dispatch completed with MCP error result",
                    );
                } else {
                    log_tool_dispatch_success("mcp", name, session_id);
                }
                Ok(if payload.is_error {
                    ToolCallOutput::error(payload.text, tool_call_id)
                } else {
                    ToolCallOutput::success(payload.text, tool_call_id)
                })
            }
            xiuxian_mcp::McpToolCallExecution::Timeout { detail } => {
                if let Some(detail) = detail {
                    log_tool_dispatch_timeout_with_detail(
                        "mcp",
                        name,
                        session_id,
                        timeout_secs,
                        &detail,
                    );
                } else {
                    log_tool_dispatch_timeout("mcp", name, session_id, timeout_secs);
                }
                Ok(tool_timeout_error_output(
                    "mcp",
                    name,
                    timeout_secs,
                    tool_call_id,
                ))
            }
            xiuxian_mcp::McpToolCallExecution::TransportError(error) => {
                log_tool_dispatch_error_with_detail(
                    "mcp",
                    name,
                    session_id,
                    "transport_error",
                    &error,
                    "tool dispatch failed before MCP response",
                );
                Err(error)
            }
        }
    }
}
