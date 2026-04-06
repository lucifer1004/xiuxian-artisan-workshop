//! TOML configuration loading using xiuxian-config-core.
//!
//! This crate-private module provides one shared merged Wendao settings surface
//! so host domains such as `link_graph` and `memory` can resolve runtime state
//! from the same source of truth.

use super::overrides::wendao_config_file_override;
use serde_yaml::Value;
use std::path::Path;
use xiuxian_config_core::{
    ConfigCascadeSpec, load_toml_value_with_imports, resolve_and_merge_toml,
};

const EMBEDDED_WENDAO_TOML: &str = include_str!("../../resources/config/wendao.toml");

pub(crate) fn merged_wendao_settings() -> Value {
    let spec = ConfigCascadeSpec::new("link_graph", EMBEDDED_WENDAO_TOML, "wendao.toml")
        .with_embedded_source_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/config/wendao.toml"
        ));

    if let Some(user_path) = wendao_config_file_override()
        && let Ok(user_config) = load_toml_value_with_imports(user_path.as_path())
    {
        let mut merged = load_embedded_defaults();
        deep_merge(&mut merged, toml_value_to_yaml(&user_config));
        return merged;
    }

    match resolve_and_merge_toml(spec) {
        Ok(toml_value) => {
            let json_str = serde_json::to_string(&toml_value)
                .ok()
                .unwrap_or_else(|| "{}".to_string());
            serde_json::from_str::<Value>(&json_str).unwrap_or_else(|_| load_embedded_defaults())
        }
        Err(_) => load_embedded_defaults(),
    }
}

fn load_embedded_defaults() -> Value {
    let source_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/config/wendao.toml"
    ));
    if let Ok(toml_value) = load_toml_value_with_imports(source_path) {
        return toml_value_to_yaml(&toml_value);
    }

    let toml_value: toml::Value =
        toml::from_str(EMBEDDED_WENDAO_TOML).unwrap_or(toml::Value::Table(toml::map::Map::new()));
    toml_value_to_yaml(&toml_value)
}

fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Mapping(base_map), Value::Mapping(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(existing) = base_map.get_mut(&key) {
                    deep_merge(existing, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

fn toml_value_to_yaml(value: &toml::Value) -> Value {
    let json_str = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str::<Value>(&json_str).unwrap_or(Value::Null)
}
