//! External tool startup helpers exposed for integration tests.

use crate::agent::tool_startup;
use crate::{AgentConfig, ToolPoolConnectConfig};

/// Build external tool client-pool connect configuration for startup mode.
#[must_use]
pub fn startup_connect_config(config: &AgentConfig, strict_startup: bool) -> ToolPoolConnectConfig {
    tool_startup::startup_connect_config(config, strict_startup)
}
