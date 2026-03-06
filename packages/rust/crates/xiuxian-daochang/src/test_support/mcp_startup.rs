//! MCP startup helpers exposed for integration tests.

use crate::agent::mcp_startup;
use crate::{AgentConfig, McpPoolConnectConfig};

/// Build MCP pool connect configuration for startup mode.
#[must_use]
pub fn startup_connect_config(config: &AgentConfig, strict_startup: bool) -> McpPoolConnectConfig {
    mcp_startup::startup_connect_config(config, strict_startup)
}
