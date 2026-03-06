//! MCP transport/client runtime for xiuxian and xiuxian-daochang.
//!
//! Follows the [MCP protocol](https://spec.modelcontextprotocol.io/) and the same client pattern
//! as [codex-rs](https://github.com/openai/codex) rmcp-client: `serve_client(handler, transport)`
//! for the handshake, then `list_tools` / `call_tool` on the running service.

mod client;
mod config;
mod tool_call;
mod tool_policy;
mod tool_schema;

pub use client::{McpClientNotificationStats, OmniMcpClient, init_params_omni_server};
pub use config::McpServerTransportConfig;
pub use tool_call::{
    McpToolCallExecution, McpToolCallOutput, call_tool_with_timeout, execute_call_with_timeout,
};
pub use tool_policy::{
    degraded_tool_error_payload, is_timeout_error_message, timeout_tool_error_payload,
};
pub use tool_schema::{llm_tool_definition, llm_tool_definitions};
