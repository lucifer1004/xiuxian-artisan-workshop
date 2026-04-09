use super::{
    resolve_search_plane_cache_connection_target_with_lookup,
    resolve_search_plane_cache_runtime_with_lookup,
};
use crate::search::cache::SearchPlaneCacheConfig;
use serde_yaml::Value;

fn settings_from_yaml(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap_or_else(|error| panic!("settings yaml: {error}"))
}

#[test]
fn search_cache_runtime_prefers_wendao_settings_over_env() {
    let settings = settings_from_yaml(
        r"
search:
  cache:
    valkey_url: redis://127.0.0.1:6380/0
    query_ttl_seconds: 180
    autocomplete_ttl_seconds: 420
    repo_revision_retention: 64
    connection_timeout_ms: 40
    response_timeout_ms: 50
",
    );

    let runtime = resolve_search_plane_cache_runtime_with_lookup(&settings, &|name| match name {
        "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL" => Some("redis://127.0.0.1:6379/9".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC" => Some("90".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_AUTOCOMPLETE_CACHE_TTL_SEC" => Some("300".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_REPO_REVISION_RETENTION" => Some("32".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS"
        | "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_RESPONSE_TIMEOUT_MS" => Some("25".to_string()),
        _ => None,
    });

    assert!(
        runtime.client.is_some(),
        "valid settings valkey url should enable the cache runtime"
    );
    assert_eq!(
        runtime.valkey_url,
        Some("redis://127.0.0.1:6380/0".to_string())
    );
    assert_eq!(
        runtime.config,
        SearchPlaneCacheConfig {
            query_ttl_seconds: 180,
            autocomplete_ttl_seconds: 420,
            repo_revision_retention: 64,
            connection_timeout: std::time::Duration::from_millis(40),
            response_timeout: std::time::Duration::from_millis(50),
        }
    );
}

#[test]
fn search_cache_runtime_falls_back_to_env_when_settings_are_missing() {
    let runtime =
        resolve_search_plane_cache_runtime_with_lookup(&Value::Null, &|name| match name {
            "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL" => {
                Some("redis://127.0.0.1:6379/3".to_string())
            }
            "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC" => Some("120".to_string()),
            _ => None,
        });

    assert!(runtime.client.is_some(), "env valkey url should still work");
    assert_eq!(
        runtime.valkey_url.as_deref(),
        Some("redis://127.0.0.1:6379/3")
    );
    assert_eq!(runtime.config.query_ttl_seconds, 120);
    assert_eq!(runtime.config.autocomplete_ttl_seconds, 300);
}

#[test]
fn search_cache_runtime_falls_back_to_env_when_settings_url_is_invalid() {
    let settings = settings_from_yaml(
        r"
search:
  cache:
    valkey_url: definitely-not-a-redis-url
",
    );

    let runtime = resolve_search_plane_cache_runtime_with_lookup(&settings, &|name| match name {
        "VALKEY_URL" => Some("redis://127.0.0.1:6379/4".to_string()),
        _ => None,
    });

    assert!(
        runtime.client.is_some(),
        "valid env should recover the optional cache runtime"
    );
    assert_eq!(
        runtime.valkey_url,
        Some("redis://127.0.0.1:6379/4".to_string())
    );
}

#[test]
fn search_cache_connection_target_reports_missing_configuration() {
    let error =
        match resolve_search_plane_cache_connection_target_with_lookup(&Value::Null, &|_| None) {
            Ok(target) => {
                panic!("missing configuration should not resolve a connection target: {target:?}")
            }
            Err(error) => error,
        };

    assert!(error.contains("missing search cache valkey url"));
}

#[test]
fn search_cache_connection_target_reports_invalid_toml_configuration() {
    let settings = settings_from_yaml(
        r"
search:
  cache:
    valkey_url: definitely-not-a-redis-url
",
    );

    let error = match resolve_search_plane_cache_connection_target_with_lookup(&settings, &|_| None)
    {
        Ok(target) => panic!("invalid TOML should not resolve a connection target: {target:?}"),
        Err(error) => error,
    };

    assert!(error.contains("invalid search.cache.valkey_url"));
}

#[test]
fn search_cache_connection_target_reports_valid_env_fallback() {
    let target =
        resolve_search_plane_cache_connection_target_with_lookup(
            &Value::Null,
            &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1:6379/4".to_string()),
                _ => None,
            },
        )
        .unwrap_or_else(|error| panic!("env fallback target should resolve: {error}"));

    assert_eq!(target.valkey_url, "redis://127.0.0.1:6379/4".to_string());
}

#[test]
fn search_cache_connection_target_reports_invalid_env_source_name() {
    let error =
        match resolve_search_plane_cache_connection_target_with_lookup(&Value::Null, &|name| {
            match name {
                "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL" => Some("   ".to_string()),
                "VALKEY_URL" => Some(" definitely-not-a-redis-url ".to_string()),
                _ => None,
            }
        }) {
            Ok(target) => {
                panic!("invalid env fallback should not resolve a connection target: {target:?}")
            }
            Err(error) => error,
        };

    assert!(error.contains("VALKEY_URL"));
}
