#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::field_reassign_with_default,
    clippy::manual_async_fn,
    clippy::async_yields_async,
    clippy::no_effect_underscore_binding
)]

use xiuxian_llm::mcp::{DiscoverCacheConfig, DiscoverReadThroughCache};

#[test]
fn discover_cache_build_key_rejects_non_discover_tools() {
    let cache = DiscoverReadThroughCache::from_config(DiscoverCacheConfig {
        valkey_url: "redis://127.0.0.1:6379/".to_string(),
        key_prefix: "omni-agent:discover".to_string(),
        ttl_secs: 30,
    })
    .expect("build cache");

    let args = serde_json::json!({"intent": "git commit", "limit": 5});
    assert!(cache.build_cache_key("git.commit", Some(&args)).is_none());
}

#[test]
fn discover_cache_build_key_is_canonical_for_argument_order() {
    let cache = DiscoverReadThroughCache::from_config(DiscoverCacheConfig {
        valkey_url: "redis://127.0.0.1:6379/".to_string(),
        key_prefix: "omni-agent:discover".to_string(),
        ttl_secs: 30,
    })
    .expect("build cache");

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
        .expect("key a");
    let key_b = cache
        .build_cache_key("skill.discover", Some(&args_b))
        .expect("key b");
    assert_eq!(key_a, key_b);
}
