use super::{
    DEFAULT_KNOWLEDGE_VALKEY_URL, KnowledgeStorage, resolve_knowledge_valkey_url_with_fallback,
    resolve_knowledge_valkey_url_with_settings_and_lookup,
};
use serde_yaml::Value;

fn settings_from_yaml(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap_or_else(|error| panic!("settings yaml: {error}"))
}

#[test]
fn resolve_knowledge_valkey_url_uses_localhost_fallback() {
    assert_eq!(
        resolve_knowledge_valkey_url_with_fallback(None),
        DEFAULT_KNOWLEDGE_VALKEY_URL.to_string()
    );
}

#[test]
fn resolve_knowledge_valkey_url_preserves_trimmed_env_choice() {
    assert_eq!(
        resolve_knowledge_valkey_url_with_fallback(Some("redis://127.0.0.1/1".to_string())),
        "redis://127.0.0.1/1".to_string()
    );
}

#[test]
fn resolve_knowledge_valkey_url_prefers_toml_over_env() {
    let settings = settings_from_yaml(
        r#"
storage:
  knowledge:
    valkey_url: "redis://127.0.0.1:6380/0"
"#,
    );

    let url = resolve_knowledge_valkey_url_with_settings_and_lookup(&settings, &|_| {
        Some("redis://127.0.0.1:6379/9".to_string())
    });

    assert_eq!(url, "redis://127.0.0.1:6380/0");
}

#[test]
fn resolve_knowledge_valkey_url_falls_back_to_env_when_toml_is_missing() {
    let url =
        resolve_knowledge_valkey_url_with_settings_and_lookup(&Value::Null, &|name| match name {
            "XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL" => Some("redis://127.0.0.1:6379/4".to_string()),
            _ => None,
        });

    assert_eq!(url, "redis://127.0.0.1:6379/4");
}

#[test]
fn resolve_knowledge_valkey_url_keeps_invalid_toml_authoritative() {
    let settings = settings_from_yaml(
        r#"
storage:
  knowledge:
    valkey_url: " definitely-not-a-redis-url "
"#,
    );

    let url = resolve_knowledge_valkey_url_with_settings_and_lookup(&settings, &|_| {
        Some("redis://127.0.0.1:6379/9".to_string())
    });

    assert_eq!(url, "definitely-not-a-redis-url");
}

#[test]
fn redis_client_opens_trimmed_valid_url() {
    let client = KnowledgeStorage::redis_client_from_url(" redis://127.0.0.1/ ");
    assert!(client.is_ok());
}
