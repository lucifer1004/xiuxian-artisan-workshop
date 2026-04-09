use super::SearchPlaneCacheConfig;
use serde_yaml::Value;

fn settings_from_yaml(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap_or_else(|error| panic!("settings yaml: {error}"))
}

#[test]
fn search_cache_config_prefers_toml_values_over_env() {
    let settings = settings_from_yaml(
        r"
search:
  cache:
    query_ttl_seconds: 180
    autocomplete_ttl_seconds: 420
    repo_revision_retention: 64
    connection_timeout_ms: 40
    response_timeout_ms: 50
",
    );

    let config = SearchPlaneCacheConfig::from_settings_and_env(&settings, &|name| match name {
        "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC" => Some("90".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_AUTOCOMPLETE_CACHE_TTL_SEC" => Some("300".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_REPO_REVISION_RETENTION" => Some("32".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS"
        | "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_RESPONSE_TIMEOUT_MS" => Some("25".to_string()),
        _ => None,
    });

    assert_eq!(config.query_ttl_seconds, 180);
    assert_eq!(config.autocomplete_ttl_seconds, 420);
    assert_eq!(config.repo_revision_retention, 64);
    assert_eq!(
        config.connection_timeout,
        std::time::Duration::from_millis(40)
    );
    assert_eq!(
        config.response_timeout,
        std::time::Duration::from_millis(50)
    );
}

#[test]
fn search_cache_config_falls_back_to_env_when_toml_value_is_invalid() {
    let settings = settings_from_yaml(
        r"
search:
  cache:
    query_ttl_seconds: invalid
    connection_timeout_ms: nope
",
    );

    let config = SearchPlaneCacheConfig::from_settings_and_env(&settings, &|name| match name {
        "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC" => Some("120".to_string()),
        "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS" => Some("35".to_string()),
        _ => None,
    });

    assert_eq!(config.query_ttl_seconds, 120);
    assert_eq!(
        config.connection_timeout,
        std::time::Duration::from_millis(35)
    );
}
