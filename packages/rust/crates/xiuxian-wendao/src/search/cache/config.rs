use crate::settings::get_setting_string;
use serde_yaml::Value;
use std::time::Duration;

const QUERY_CACHE_TTL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC";
const AUTOCOMPLETE_CACHE_TTL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_AUTOCOMPLETE_CACHE_TTL_SEC";
const REPO_REVISION_RETENTION_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_REPO_REVISION_RETENTION";
const CACHE_CONNECTION_TIMEOUT_MS_ENV: &str =
    "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS";
const CACHE_RESPONSE_TIMEOUT_MS_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_RESPONSE_TIMEOUT_MS";
const QUERY_CACHE_TTL_SETTING: &str = "search.cache.query_ttl_seconds";
const AUTOCOMPLETE_CACHE_TTL_SETTING: &str = "search.cache.autocomplete_ttl_seconds";
const REPO_REVISION_RETENTION_SETTING: &str = "search.cache.repo_revision_retention";
const CACHE_CONNECTION_TIMEOUT_MS_SETTING: &str = "search.cache.connection_timeout_ms";
const CACHE_RESPONSE_TIMEOUT_MS_SETTING: &str = "search.cache.response_timeout_ms";

const DEFAULT_QUERY_CACHE_TTL_SEC: u64 = 90;
const DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC: u64 = 300;
const DEFAULT_REPO_REVISION_RETENTION: usize = 32;
const DEFAULT_CACHE_CONNECTION_TIMEOUT_MS: u64 = 25;
const DEFAULT_CACHE_RESPONSE_TIMEOUT_MS: u64 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchPlaneCacheTtl {
    HotQuery,
    Autocomplete,
}

impl SearchPlaneCacheTtl {
    pub(crate) fn as_seconds(self, config: &SearchPlaneCacheConfig) -> u64 {
        match self {
            Self::HotQuery => config.query_ttl_seconds,
            Self::Autocomplete => config.autocomplete_ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchPlaneCacheConfig {
    pub(crate) query_ttl_seconds: u64,
    pub(crate) autocomplete_ttl_seconds: u64,
    pub(crate) repo_revision_retention: usize,
    pub(crate) connection_timeout: Duration,
    pub(crate) response_timeout: Duration,
}

impl Default for SearchPlaneCacheConfig {
    fn default() -> Self {
        Self {
            query_ttl_seconds: DEFAULT_QUERY_CACHE_TTL_SEC,
            autocomplete_ttl_seconds: DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC,
            repo_revision_retention: DEFAULT_REPO_REVISION_RETENTION,
            connection_timeout: Duration::from_millis(DEFAULT_CACHE_CONNECTION_TIMEOUT_MS),
            response_timeout: Duration::from_millis(DEFAULT_CACHE_RESPONSE_TIMEOUT_MS),
        }
    }
}

impl SearchPlaneCacheConfig {
    pub(crate) fn from_settings_and_env(
        settings: &Value,
        lookup: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        Self {
            query_ttl_seconds: parse_setting_u64(settings, QUERY_CACHE_TTL_SETTING)
                .or_else(|| parse_lookup_u64(QUERY_CACHE_TTL_ENV, lookup))
                .unwrap_or(DEFAULT_QUERY_CACHE_TTL_SEC),
            autocomplete_ttl_seconds: parse_setting_u64(settings, AUTOCOMPLETE_CACHE_TTL_SETTING)
                .or_else(|| parse_lookup_u64(AUTOCOMPLETE_CACHE_TTL_ENV, lookup))
                .unwrap_or(DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC),
            repo_revision_retention: parse_setting_u64(settings, REPO_REVISION_RETENTION_SETTING)
                .or_else(|| parse_lookup_u64(REPO_REVISION_RETENTION_ENV, lookup))
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value > 0)
                .unwrap_or(DEFAULT_REPO_REVISION_RETENTION),
            connection_timeout: Duration::from_millis(
                parse_setting_u64(settings, CACHE_CONNECTION_TIMEOUT_MS_SETTING)
                    .or_else(|| parse_lookup_u64(CACHE_CONNECTION_TIMEOUT_MS_ENV, lookup))
                    .unwrap_or(DEFAULT_CACHE_CONNECTION_TIMEOUT_MS),
            ),
            response_timeout: Duration::from_millis(
                parse_setting_u64(settings, CACHE_RESPONSE_TIMEOUT_MS_SETTING)
                    .or_else(|| parse_lookup_u64(CACHE_RESPONSE_TIMEOUT_MS_ENV, lookup))
                    .unwrap_or(DEFAULT_CACHE_RESPONSE_TIMEOUT_MS),
            ),
        }
    }
}

fn parse_setting_u64(settings: &Value, dotted_key: &str) -> Option<u64> {
    get_setting_string(settings, dotted_key)
        .as_deref()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn parse_lookup_u64(name: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Option<u64> {
    lookup(name).and_then(|value| value.trim().parse::<u64>().ok())
}
