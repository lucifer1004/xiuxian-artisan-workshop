#![allow(
    missing_docs,
    unused_imports,
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::field_reassign_with_default,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::map_unwrap_or,
    clippy::unreadable_literal,
    clippy::useless_conversion,
    clippy::match_wildcard_for_single_variants,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_raw_string_hashes,
    clippy::manual_async_fn,
    clippy::manual_let_else,
    clippy::too_many_lines,
    clippy::unnecessary_literal_bound,
    clippy::needless_pass_by_value,
    clippy::struct_field_names,
    clippy::single_match_else,
    clippy::assigning_clones
)]

//! Unit tests: config and session store (no network).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use xiuxian_daochang::{
    AgentConfig, ChatMessage, ContextBudgetStrategy, MemoryConfig, SessionStore,
    set_config_home_override,
};

fn runtime_settings_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn runtime_settings_test_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = std::env::temp_dir()
            .join("xiuxian-daochang-tests")
            .join(format!("config-and-session-{}", std::process::id()));
        std::fs::create_dir_all(root.join("xiuxian-artisan-workshop"))
            .unwrap_or_else(|error| panic!("create test config root: {error}"));
        set_config_home_override(root.clone());
        root
    })
}

fn write_runtime_settings_override(raw: &str) {
    let root = runtime_settings_test_root();
    let path = root.join("xiuxian-artisan-workshop").join("xiuxian.toml");
    std::fs::write(&path, raw).unwrap_or_else(|error| {
        panic!(
            "write runtime settings override {}: {error}",
            path.display()
        )
    });
}

#[test]
fn config_resolve_api_key_from_field() {
    let config = AgentConfig {
        api_key: Some("sk-test".to_string()),
        ..Default::default()
    };
    assert_eq!(config.resolve_api_key().as_deref(), Some("sk-test"));
}

#[test]
fn config_resolve_api_key_prefers_matching_runtime_settings_env_reference() {
    let _guard = runtime_settings_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    write_runtime_settings_override(
        r#"
[llm]
default_provider = "openai"

[llm.providers.openai]
base_url = "https://token-plan-sgp.xiaomimimo.com/v1"
api_key = "MIMO_API_KEY"
model = "mimo-v2-pro"
"#,
    );
    let config = AgentConfig {
        inference_url: "https://token-plan-sgp.xiaomimimo.com/v1/chat/completions".to_string(),
        api_key: None,
        ..Default::default()
    };

    let resolved = config.resolve_api_key_with_env_reader(|key| match key {
        "MIMO_API_KEY" => Some("tp-test".to_string()),
        "OPENAI_API_KEY" => Some("sk-should-not-win".to_string()),
        _ => None,
    });

    assert_eq!(resolved.as_deref(), Some("tp-test"));
}

#[test]
fn config_resolve_api_key_keeps_url_heuristic_for_non_matching_runtime_settings() {
    let _guard = runtime_settings_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    write_runtime_settings_override(
        r#"
[llm]
default_provider = "openai"

[llm.providers.openai]
base_url = "https://token-plan-sgp.xiaomimimo.com/v1"
api_key = "MIMO_API_KEY"
model = "mimo-v2-pro"
"#,
    );
    let config = AgentConfig {
        inference_url: "https://api.openai.com/v1/chat/completions".to_string(),
        api_key: None,
        ..Default::default()
    };

    let resolved = config.resolve_api_key_with_env_reader(|key| match key {
        "OPENAI_API_KEY" => Some("sk-openai".to_string()),
        "MIMO_API_KEY" => Some("tp-should-not-win".to_string()),
        _ => None,
    });

    assert_eq!(resolved.as_deref(), Some("sk-openai"));
}

#[test]
fn config_default_tool_servers_empty() {
    let config = AgentConfig::default();
    assert!(config.tool_servers.is_empty());
    assert_eq!(config.tool_pool_size, 4);
    assert_eq!(config.tool_handshake_timeout_secs, 30);
    assert_eq!(config.tool_connect_retries, 3);
    assert!(config.tool_strict_startup);
    assert_eq!(config.tool_connect_retry_backoff_ms, 1_000);
    assert_eq!(config.tool_timeout_secs, 180);
    assert_eq!(config.tool_list_cache_ttl_ms, 1_000);
    assert_eq!(config.max_tool_rounds, 30);
    assert_eq!(
        config.context_budget_strategy,
        ContextBudgetStrategy::RecentFirst
    );
}

#[test]
fn memory_config_default_gate_policy_matches_runtime_defaults() {
    let memory = MemoryConfig::default();
    assert_eq!(memory.gate_promote_threshold, 0.78);
    assert_eq!(memory.gate_obsolete_threshold, 0.32);
    assert_eq!(memory.gate_promote_min_usage, 3);
    assert_eq!(memory.gate_obsolete_min_usage, 2);
    assert_eq!(memory.gate_promote_failure_rate_ceiling, 0.25);
    assert_eq!(memory.gate_obsolete_failure_rate_floor, 0.70);
    assert_eq!(memory.gate_promote_min_ttl_score, 0.50);
    assert_eq!(memory.gate_obsolete_max_ttl_score, 0.45);
}

#[tokio::test]
async fn session_append_and_get() -> anyhow::Result<()> {
    let store = SessionStore::new()?;
    store
        .append(
            "s1",
            vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: Some("hi".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                ChatMessage {
                    role: "assistant".to_string(),
                    content: Some("hello".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
            ],
        )
        .await?;
    let msgs = store.get("s1").await?;
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].content.as_deref(), Some("hi"));
    assert_eq!(msgs[1].content.as_deref(), Some("hello"));
    Ok(())
}

#[tokio::test]
async fn session_clear() -> anyhow::Result<()> {
    let store = SessionStore::new()?;
    store
        .append(
            "s2",
            vec![ChatMessage {
                role: "user".to_string(),
                content: Some("x".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
        )
        .await?;
    assert_eq!(store.get("s2").await?.len(), 1);
    store.clear("s2").await?;
    assert!(store.get("s2").await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn session_replace_overwrites_existing_messages() -> anyhow::Result<()> {
    let store = SessionStore::new()?;
    store
        .append(
            "s3",
            vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: Some("first".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                ChatMessage {
                    role: "assistant".to_string(),
                    content: Some("reply-first".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
            ],
        )
        .await?;
    store
        .replace(
            "s3",
            vec![ChatMessage {
                role: "system".to_string(),
                content: Some("replaced".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: Some("replace-test".to_string()),
            }],
        )
        .await?;
    let messages = store.get("s3").await?;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content.as_deref(), Some("replaced"));
    Ok(())
}

#[tokio::test]
async fn session_replace_empty_clears_existing_messages() -> anyhow::Result<()> {
    let store = SessionStore::new()?;
    store
        .append(
            "s4",
            vec![ChatMessage {
                role: "user".to_string(),
                content: Some("to-clear".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
        )
        .await?;
    assert_eq!(store.get("s4").await?.len(), 1);
    store.replace("s4", Vec::new()).await?;
    assert!(store.get("s4").await?.is_empty());
    Ok(())
}
