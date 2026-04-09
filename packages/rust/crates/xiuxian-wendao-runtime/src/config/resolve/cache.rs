use crate::config::LinkGraphCacheRuntimeConfig;
use crate::config::constants::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, LINK_GRAPH_CACHE_VALKEY_URL_ENV,
    LINK_GRAPH_VALKEY_KEY_PREFIX_ENV, LINK_GRAPH_VALKEY_TTL_SECONDS_ENV,
};
use crate::settings::get_setting_string;
use serde_yaml::Value;
use xiuxian_config_core::{toml_first_env, toml_first_named_string};

const LINK_GRAPH_CACHE_VALKEY_URL_SETTING: &str = "link_graph.cache.valkey_url";
const LINK_GRAPH_CACHE_KEY_PREFIX_SETTING: &str = "link_graph.cache.key_prefix";
const LINK_GRAPH_CACHE_TTL_SECONDS_SETTING: &str = "link_graph.cache.ttl_seconds";

/// Resolve runtime cache configuration from merged settings and environment.
///
/// # Errors
///
/// Returns an error when no Valkey URL can be resolved from config or env.
pub fn resolve_link_graph_cache_runtime_with_settings(
    settings: &Value,
) -> Result<LinkGraphCacheRuntimeConfig, String> {
    resolve_link_graph_cache_runtime_with_settings_and_lookup(settings, &|name| {
        std::env::var(name).ok()
    })
}

fn resolve_link_graph_cache_runtime_with_settings_and_lookup(
    settings: &Value,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<LinkGraphCacheRuntimeConfig, String> {
    let valkey_url = toml_first_named_string(
        LINK_GRAPH_CACHE_VALKEY_URL_SETTING,
        get_setting_string(settings, LINK_GRAPH_CACHE_VALKEY_URL_SETTING),
        lookup,
        &[LINK_GRAPH_CACHE_VALKEY_URL_ENV],
    )
    .map(|(_, value)| value)
    .ok_or_else(|| {
        format!(
            "link_graph cache valkey url is required (set {LINK_GRAPH_CACHE_VALKEY_URL_SETTING} or {LINK_GRAPH_CACHE_VALKEY_URL_ENV})"
        )
    })?;

    let key_prefix = toml_first_env!(
        settings,
        LINK_GRAPH_CACHE_KEY_PREFIX_SETTING,
        lookup,
        [LINK_GRAPH_VALKEY_KEY_PREFIX_ENV],
        get_setting_string
    )
    .unwrap_or_else(|| DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string());

    let ttl_seconds = toml_first_env!(
        settings,
        LINK_GRAPH_CACHE_TTL_SECONDS_SETTING,
        lookup,
        [LINK_GRAPH_VALKEY_TTL_SECONDS_ENV],
        get_setting_string,
        xiuxian_config_core::parse_positive::<u64>
    );

    Ok(LinkGraphCacheRuntimeConfig::from_parts(
        &valkey_url,
        Some(&key_prefix),
        ttl_seconds,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_link_graph_cache_runtime_with_settings,
        resolve_link_graph_cache_runtime_with_settings_and_lookup,
    };
    use crate::config::test_support;
    use serde_yaml::Value;
    use std::fs;

    #[test]
    fn resolve_cache_runtime_reads_override_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.cache]
valkey_url = "redis://127.0.0.1:6379/1"
key_prefix = "custom:key"
ttl_seconds = 120
"#,
        )?;

        let settings = test_support::load_test_settings_from_path(&config_path)?;
        let runtime = resolve_link_graph_cache_runtime_with_settings(&settings)?;
        assert_eq!(runtime.valkey_url, "redis://127.0.0.1:6379/1");
        assert_eq!(runtime.key_prefix, "custom:key");
        assert_eq!(runtime.ttl_seconds, Some(120));

        Ok(())
    }

    #[test]
    fn resolve_cache_runtime_falls_back_to_env_when_toml_is_missing()
    -> Result<(), Box<dyn std::error::Error>> {
        let settings = Value::Null;
        let runtime = resolve_link_graph_cache_runtime_with_settings_and_lookup(
            &settings,
            &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1:6379/7".to_string()),
                _ => None,
            },
        )?;
        assert_eq!(runtime.valkey_url, "redis://127.0.0.1:6379/7");
        Ok(())
    }

    #[test]
    fn resolve_cache_runtime_keeps_invalid_toml_authoritative()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.cache]
valkey_url = " definitely-not-a-redis-url "
ttl_seconds = "invalid"
"#,
        )?;
        let settings = test_support::load_test_settings_from_path(&config_path)?;

        let runtime = resolve_link_graph_cache_runtime_with_settings_and_lookup(
            &settings,
            &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1:6379/8".to_string()),
                _ => None,
            },
        )?;
        assert_eq!(runtime.valkey_url, "definitely-not-a-redis-url");
        assert_eq!(runtime.ttl_seconds, None);
        Ok(())
    }
}
