//! External tool config loader.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use super::agent::ToolServerEntry;

/// Top-level tool config shape: `{ "toolServers": { "<name>": { ... } } }`.
#[derive(Debug, Deserialize)]
pub struct ToolConfigFile {
    /// Map of server name to server config (http URL or stdio command/args).
    #[serde(rename = "toolServers")]
    pub tool_servers: Option<std::collections::HashMap<String, ToolServerEntryFile>>,
}

/// Per-server entry in the tool config surface (`http` | `stdio`).
#[derive(Debug, Deserialize)]
pub struct ToolServerEntryFile {
    /// Transport type: "http" or "stdio".
    #[serde(rename = "type")]
    pub typ: Option<String>,
    /// For http: base URL (e.g. `http://localhost:3002`).
    pub url: Option<String>,
    /// For stdio: command to run (e.g. `omni`).
    pub command: Option<String>,
    /// For stdio: command arguments.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Load external tool server entries from the tool config file surface. No env fallback.
///
/// Returns empty list if the file is missing or has no `toolServers`.
///
/// # Errors
/// Returns an error when file read or JSON parse fails.
pub fn load_tool_config(path: &Path) -> Result<Vec<ToolServerEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).context("read tool config file")?;
    let file: ToolConfigFile = serde_json::from_slice(&bytes).context("parse tool config JSON")?;
    let servers = file.tool_servers.unwrap_or_default();
    let out: Vec<ToolServerEntry> = servers
        .into_iter()
        .map(|(name, e)| file_entry_to_tool_server_entry(name, e))
        .collect();
    Ok(out)
}

fn file_entry_to_tool_server_entry(name: String, e: ToolServerEntryFile) -> ToolServerEntry {
    let typ = e.typ.as_deref().unwrap_or("http");
    if typ == "stdio" {
        ToolServerEntry {
            name: name.clone(),
            url: None,
            command: e.command.or(Some("omni".to_string())),
            args: (!e.args.is_empty()).then_some(e.args),
        }
    } else {
        // Preserve configured HTTP URL exactly (trim + remove trailing slash only).
        // This supports both legacy `/sse` endpoints and newer root/message routes.
        let url = e
            .url
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty());
        ToolServerEntry {
            name,
            url,
            command: None,
            args: None,
        }
    }
}
