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
    let runtime =
        resolve_link_graph_cache_runtime_with_settings_and_lookup(&settings, &|name| match name {
            "VALKEY_URL" => Some("redis://127.0.0.1:6379/7".to_string()),
            _ => None,
        })?;
    assert_eq!(runtime.valkey_url, "redis://127.0.0.1:6379/7");
    Ok(())
}

#[test]
fn resolve_cache_runtime_keeps_invalid_toml_authoritative() -> Result<(), Box<dyn std::error::Error>>
{
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

    let runtime =
        resolve_link_graph_cache_runtime_with_settings_and_lookup(&settings, &|name| match name {
            "VALKEY_URL" => Some("redis://127.0.0.1:6379/8".to_string()),
            _ => None,
        })?;
    assert_eq!(runtime.valkey_url, "definitely-not-a-redis-url");
    assert_eq!(runtime.ttl_seconds, None);
    Ok(())
}
