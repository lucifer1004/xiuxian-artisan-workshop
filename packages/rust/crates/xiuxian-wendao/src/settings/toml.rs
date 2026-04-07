//! TOML configuration loading using xiuxian-config-core.
//!
//! This crate-private module provides one shared merged Wendao settings surface
//! so host domains such as `link_graph` and `memory` can resolve runtime state
//! from the same source of truth.

use super::overrides::{wendao_config_file_override, wendao_config_home_override};
use serde_yaml::Value;
use std::path::{Path, PathBuf};
use xiuxian_config_core::{
    ArrayMergeStrategy, ConfigCascadeSpec, load_toml_value_with_imports_and_paths,
    merge_toml_values, resolve_and_merge_toml_with_paths, resolve_config_home,
    resolve_project_root,
};

const EMBEDDED_WENDAO_TOML: &str = include_str!("../../resources/config/wendao.toml");
const EMBEDDED_WENDAO_TOML_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/resources/config/wendao.toml");

pub(crate) fn merged_wendao_settings() -> Value {
    toml_value_to_yaml(&merged_wendao_toml())
}

fn merged_wendao_toml() -> toml::Value {
    let spec = wendao_settings_spec();
    let project_root = resolve_project_root();
    let config_home = resolved_wendao_config_home(project_root.as_deref());

    if let Some(user_path) = wendao_config_file_override()
        && let Ok(user_config) = load_toml_value_with_imports_and_paths(
            user_path.as_path(),
            project_root.as_deref(),
            config_home.as_deref(),
        )
    {
        let mut merged =
            load_embedded_defaults_toml(project_root.as_deref(), config_home.as_deref());
        merge_toml_values(&mut merged, user_config, ArrayMergeStrategy::Overwrite);
        return merged;
    }

    resolve_and_merge_toml_with_paths(spec, project_root.as_deref(), config_home.as_deref())
        .unwrap_or_else(|_| {
            load_embedded_defaults_toml(project_root.as_deref(), config_home.as_deref())
        })
}

fn wendao_settings_spec() -> ConfigCascadeSpec<'static> {
    ConfigCascadeSpec::new("link_graph", EMBEDDED_WENDAO_TOML, "wendao.toml")
        .with_embedded_source_path(EMBEDDED_WENDAO_TOML_PATH)
}

fn load_embedded_defaults_toml(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> toml::Value {
    let source_path = Path::new(EMBEDDED_WENDAO_TOML_PATH);
    if let Ok(toml_value) =
        load_toml_value_with_imports_and_paths(source_path, project_root, config_home)
    {
        return toml_value;
    }

    let toml_value: toml::Value =
        toml::from_str(EMBEDDED_WENDAO_TOML).unwrap_or(toml::Value::Table(toml::map::Map::new()));
    toml_value
}

fn resolved_wendao_config_home(project_root: Option<&Path>) -> Option<PathBuf> {
    wendao_config_home_override().or_else(|| resolve_config_home(project_root))
}

fn toml_value_to_yaml(value: &toml::Value) -> Value {
    let json_str = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str::<Value>(&json_str).unwrap_or(Value::Null)
}
