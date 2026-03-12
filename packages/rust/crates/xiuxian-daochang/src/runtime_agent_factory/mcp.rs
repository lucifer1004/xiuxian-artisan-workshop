use std::path::Path;

use crate::{
    McpServerEntry, RuntimeSettings,
    env_parse::{
        parse_bool_from_env, parse_positive_u32_from_env, parse_positive_u64_from_env,
        parse_positive_usize_from_env,
    },
    load_mcp_config,
};
use anyhow::Result;
use xiuxian_mcp::{
    McpRuntimeConfig, McpRuntimeOptions, resolve_mcp_runtime_config, streamable_http_server_entries,
};

pub(crate) fn resolve_runtime_mcp_servers(mcp_config_path: &Path) -> Result<Vec<McpServerEntry>> {
    Ok(streamable_http_server_entries(&load_mcp_config(
        mcp_config_path,
    )?))
}

pub(crate) fn resolve_runtime_mcp_options(runtime_settings: &RuntimeSettings) -> McpRuntimeOptions {
    resolve_mcp_runtime_config(McpRuntimeConfig {
        pool_size: parse_positive_usize_from_env("OMNI_AGENT_MCP_POOL_SIZE")
            .or(runtime_settings.mcp.pool_size),
        handshake_timeout_secs: parse_positive_u64_from_env(
            "OMNI_AGENT_MCP_HANDSHAKE_TIMEOUT_SECS",
        )
        .or(runtime_settings.mcp.handshake_timeout_secs),
        connect_retries: parse_positive_u32_from_env("OMNI_AGENT_MCP_CONNECT_RETRIES")
            .or(runtime_settings.mcp.connect_retries),
        strict_startup: parse_bool_from_env("OMNI_AGENT_MCP_STRICT_STARTUP")
            .or(runtime_settings.mcp.strict_startup),
        connect_retry_backoff_ms: parse_positive_u64_from_env(
            "OMNI_AGENT_MCP_CONNECT_RETRY_BACKOFF_MS",
        )
        .or(runtime_settings.mcp.connect_retry_backoff_ms),
        tool_timeout_secs: parse_positive_u64_from_env("OMNI_AGENT_MCP_TOOL_TIMEOUT_SECS")
            .or(runtime_settings.mcp.tool_timeout_secs),
        list_tools_cache_ttl_ms: parse_positive_u64_from_env(
            "OMNI_AGENT_MCP_LIST_TOOLS_CACHE_TTL_MS",
        )
        .or(runtime_settings.mcp.list_tools_cache_ttl_ms),
    })
}
