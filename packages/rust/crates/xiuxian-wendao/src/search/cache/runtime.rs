use serde_yaml::Value;

use crate::settings::{get_setting_string, merged_wendao_settings};
use crate::valkey_common::open_client;

use super::config::SearchPlaneCacheConfig;

const SEARCH_CACHE_VALKEY_URL_SETTING: &str = "search.cache.valkey_url";
const SEARCH_PLANE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL";
const KNOWLEDGE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL";
const VALKEY_URL_ENV: &str = "VALKEY_URL";
const REDIS_URL_ENV: &str = "REDIS_URL";

#[derive(Debug, Clone)]
pub(crate) struct SearchPlaneCacheRuntime {
    pub(crate) client: Option<redis::Client>,
    pub(crate) config: SearchPlaneCacheConfig,
}

pub(crate) fn resolve_search_plane_cache_runtime() -> SearchPlaneCacheRuntime {
    let settings = merged_wendao_settings();
    resolve_search_plane_cache_runtime_with_lookup(&settings, &|name| std::env::var(name).ok())
}

fn resolve_search_plane_cache_runtime_with_lookup(
    settings: &Value,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> SearchPlaneCacheRuntime {
    SearchPlaneCacheRuntime {
        client: resolve_valkey_client_with_lookup(settings, lookup),
        config: SearchPlaneCacheConfig::from_settings_and_env(settings, lookup),
    }
}

fn resolve_valkey_client_with_lookup(
    settings: &Value,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<redis::Client> {
    resolve_valid_valkey_url_with_lookup(settings, lookup)
        .and_then(|value| open_client(value.as_str()).ok())
}

fn resolve_valid_valkey_url_with_lookup(
    settings: &Value,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    resolved_setting_string(settings, SEARCH_CACHE_VALKEY_URL_SETTING)
        .filter(|value| open_client(value.as_str()).is_ok())
        .or_else(|| {
            first_non_empty_lookup(
                &[
                    SEARCH_PLANE_VALKEY_URL_ENV,
                    KNOWLEDGE_VALKEY_URL_ENV,
                    VALKEY_URL_ENV,
                    REDIS_URL_ENV,
                ],
                lookup,
            )
            .filter(|value| open_client(value.as_str()).is_ok())
        })
}

fn resolved_setting_string(settings: &Value, dotted_key: &str) -> Option<String> {
    get_setting_string(settings, dotted_key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn first_non_empty_lookup(
    names: &[&str],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_search_plane_cache_runtime_with_lookup, resolve_valid_valkey_url_with_lookup,
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

        let runtime =
            resolve_search_plane_cache_runtime_with_lookup(&settings, &|name| match name {
                "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL" => {
                    Some("redis://127.0.0.1:6379/9".to_string())
                }
                "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC" => Some("90".to_string()),
                "XIUXIAN_WENDAO_SEARCH_PLANE_AUTOCOMPLETE_CACHE_TTL_SEC" => Some("300".to_string()),
                "XIUXIAN_WENDAO_SEARCH_PLANE_REPO_REVISION_RETENTION" => Some("32".to_string()),
                "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS"
                | "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_RESPONSE_TIMEOUT_MS" => {
                    Some("25".to_string())
                }
                _ => None,
            });

        assert!(
            runtime.client.is_some(),
            "valid settings valkey url should enable the cache runtime"
        );
        assert_eq!(
            resolve_valid_valkey_url_with_lookup(&settings, &|_| None),
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

        let runtime =
            resolve_search_plane_cache_runtime_with_lookup(&settings, &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1:6379/4".to_string()),
                _ => None,
            });

        assert!(
            runtime.client.is_some(),
            "valid env should recover the optional cache runtime"
        );
        assert_eq!(
            resolve_valid_valkey_url_with_lookup(&settings, &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1:6379/4".to_string()),
                _ => None,
            }),
            Some("redis://127.0.0.1:6379/4".to_string())
        );
    }
}
