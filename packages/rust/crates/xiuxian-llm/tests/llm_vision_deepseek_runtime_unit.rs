//! MCP discover read-through cache tests.

use std::borrow::Cow;

use anyhow::{Result, anyhow};
use xiuxian_mcp::{
    DiscoverCacheConfig, DiscoverCacheRuntimeConfig, DiscoverReadThroughCache,
    resolve_discover_cache_config,
};

#[test]
fn discover_cache_build_key_rejects_non_discover_tools() -> Result<()> {
    let cache = DiscoverReadThroughCache::from_config(DiscoverCacheConfig {
        valkey_url: "redis://127.0.0.1:6379/".to_string(),
        key_prefix: "xiuxian-daochang:discover".to_string(),
        ttl_secs: 30,
    })?;

    let args = serde_json::json!({"intent": "git commit", "limit": 5});
    assert!(cache.build_cache_key("git.commit", Some(&args)).is_none());
    Ok(())
}

#[test]
fn discover_cache_build_key_is_canonical_for_argument_order() -> Result<()> {
    let cache = DiscoverReadThroughCache::from_config(DiscoverCacheConfig {
        valkey_url: "redis://127.0.0.1:6379/".to_string(),
        key_prefix: "xiuxian-daochang:discover".to_string(),
        ttl_secs: 30,
    })?;

    let args_a = serde_json::json!({
        "intent": "research rust mcp",
        "limit": 10,
        "extra": {"b": 2, "a": 1}
    });
    let args_b = serde_json::json!({
        "limit": 10,
        "extra": {"a": 1, "b": 2},
        "intent": "research rust mcp"
    });

    let key_a = cache
        .build_cache_key("skill.discover", Some(&args_a))
        .ok_or_else(|| anyhow!("missing key for args_a"))?;
    let key_b = cache
        .build_cache_key("skill.discover", Some(&args_b))
        .ok_or_else(|| anyhow!("missing key for args_b"))?;
    assert_eq!(key_a, key_b);
    Ok(())
}

#[test]
fn discover_cache_runtime_config_returns_none_when_disabled_or_url_missing() {
    let disabled = resolve_discover_cache_config(DiscoverCacheRuntimeConfig {
        enabled: false,
        valkey_url: Some("redis://127.0.0.1:6379/0".to_string()),
        key_prefix: None,
        ttl_secs: None,
        default_key_prefix: Cow::Borrowed("xiuxian-mcp:discover"),
    });
    assert!(disabled.is_none());

    let missing_url = resolve_discover_cache_config(DiscoverCacheRuntimeConfig {
        enabled: true,
        valkey_url: None,
        key_prefix: None,
        ttl_secs: None,
        default_key_prefix: Cow::Borrowed("xiuxian-mcp:discover"),
    });
    assert!(missing_url.is_none());
}

#[test]
fn discover_cache_runtime_config_applies_defaults_and_clamps_ttl() {
    let resolved = resolve_discover_cache_config(DiscoverCacheRuntimeConfig {
        enabled: true,
        valkey_url: Some("  redis://127.0.0.1:6379/9  ".to_string()),
        key_prefix: Some("   ".to_string()),
        ttl_secs: Some(9_999),
        default_key_prefix: Cow::Borrowed("xiuxian-mcp:discover"),
    })
    .expect("config should resolve");

    assert_eq!(resolved.valkey_url, "redis://127.0.0.1:6379/9");
    assert_eq!(resolved.key_prefix, "xiuxian-mcp:discover");
    assert_eq!(resolved.ttl_secs, 3_600);
}
