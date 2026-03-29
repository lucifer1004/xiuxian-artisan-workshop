use crate::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, LINK_GRAPH_CACHE_VALKEY_URL_ENV,
    LINK_GRAPH_VALKEY_KEY_PREFIX_ENV, LINK_GRAPH_VALKEY_TTL_SECONDS_ENV,
};
use crate::runtime_config::LinkGraphCacheRuntimeConfig;
use crate::settings::{first_non_empty, get_setting_string, parse_positive_u64};
use serde_yaml::Value;

/// Resolve runtime cache configuration from merged settings and environment.
///
/// # Errors
///
/// Returns an error when no Valkey URL can be resolved from config or env.
pub fn resolve_link_graph_cache_runtime_with_settings(
    settings: &Value,
) -> Result<LinkGraphCacheRuntimeConfig, String> {
    let valkey_url = first_non_empty(&[
        get_setting_string(settings, "link_graph.cache.valkey_url"),
        std::env::var(LINK_GRAPH_CACHE_VALKEY_URL_ENV).ok(),
    ])
    .ok_or_else(|| {
        "link_graph cache valkey url is required (set VALKEY_URL or link_graph.cache.valkey_url)"
            .to_string()
    })?;

    let key_prefix = first_non_empty(&[
        get_setting_string(settings, "link_graph.cache.key_prefix"),
        std::env::var(LINK_GRAPH_VALKEY_KEY_PREFIX_ENV).ok(),
        Some(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string()),
    ])
    .unwrap_or_else(|| DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string());

    let ttl_raw = first_non_empty(&[
        get_setting_string(settings, "link_graph.cache.ttl_seconds"),
        std::env::var(LINK_GRAPH_VALKEY_TTL_SECONDS_ENV).ok(),
    ]);
    let ttl_seconds = ttl_raw.as_deref().and_then(parse_positive_u64);

    Ok(LinkGraphCacheRuntimeConfig::from_parts(
        &valkey_url,
        Some(&key_prefix),
        ttl_seconds,
    ))
}

#[cfg(test)]
mod tests {
    use super::resolve_link_graph_cache_runtime_with_settings;
    use crate::settings::{merged_toml_settings, set_link_graph_wendao_config_override};
    use serial_test::serial;
    use std::fs;

    #[test]
    #[serial]
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
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let settings = merged_toml_settings("link_graph", "", "", "wendao.toml");
        let runtime = resolve_link_graph_cache_runtime_with_settings(&settings)?;
        assert_eq!(runtime.valkey_url, "redis://127.0.0.1:6379/1");
        assert_eq!(runtime.key_prefix, "custom:key");
        assert_eq!(runtime.ttl_seconds, Some(120));

        Ok(())
    }
}
