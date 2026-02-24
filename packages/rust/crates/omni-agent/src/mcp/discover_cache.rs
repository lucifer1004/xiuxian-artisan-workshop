//! Discover tool-call read-through cache runtime wiring.
//!
//! Core cache behavior lives in `xiuxian_llm::mcp::discover_cache`; this file
//! only resolves runtime settings/environment for omni-agent.

use std::sync::Arc;

use anyhow::Result;
use xiuxian_llm::mcp::{DiscoverCacheConfig, DiscoverReadThroughCache};

use crate::config::load_runtime_settings;

const DEFAULT_DISCOVER_CACHE_KEY_PREFIX: &str = "omni-agent:discover";
const DEFAULT_DISCOVER_CACHE_TTL_SECS: u64 = 30;
const MAX_DISCOVER_CACHE_TTL_SECS: u64 = 3_600;

/// Build discover cache from env + runtime settings.
///
/// Returns `Ok(None)` when cache is disabled or no valkey url is configured.
pub(super) fn discover_cache_from_runtime() -> Result<Option<Arc<DiscoverReadThroughCache>>> {
    let Some(config) = resolve_discover_cache_config() else {
        return Ok(None);
    };
    let cache = DiscoverReadThroughCache::from_config(config)?;
    Ok(Some(Arc::new(cache)))
}

fn resolve_discover_cache_config() -> Option<DiscoverCacheConfig> {
    let settings = load_runtime_settings();

    let enabled = env_bool("OMNI_AGENT_MCP_DISCOVER_CACHE_ENABLED")
        .or(settings.mcp.agent_discover_cache_enabled)
        .unwrap_or(true);
    if !enabled {
        return None;
    }

    let valkey_url = settings
        .session
        .valkey_url
        .as_deref()
        .map(str::trim)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("VALKEY_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })?;

    let key_prefix = std::env::var("OMNI_AGENT_MCP_DISCOVER_CACHE_KEY_PREFIX")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            settings
                .mcp
                .agent_discover_cache_key_prefix
                .as_deref()
                .map(str::trim)
                .map(str::to_string)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_DISCOVER_CACHE_KEY_PREFIX.to_string());

    let ttl_secs = std::env::var("OMNI_AGENT_MCP_DISCOVER_CACHE_TTL_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .or(settings.mcp.agent_discover_cache_ttl_secs)
        .unwrap_or(DEFAULT_DISCOVER_CACHE_TTL_SECS)
        .clamp(1, MAX_DISCOVER_CACHE_TTL_SECS);

    Some(DiscoverCacheConfig {
        valkey_url,
        key_prefix,
        ttl_secs,
    })
}

fn env_bool(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
