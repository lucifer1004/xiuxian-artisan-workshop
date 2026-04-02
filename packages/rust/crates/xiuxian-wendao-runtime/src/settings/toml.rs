use super::overrides::wendao_config_file_override;
use serde_yaml::Value;
use std::path::{Path, PathBuf};
use xiuxian_config_core::{
    ConfigCascadeSpec, load_toml_value_with_imports, resolve_and_merge_toml,
};

/// Merge runtime TOML settings with cascading support.
///
/// Both embedded defaults and explicit user overrides resolve recursive
/// `imports` relative to their physical source file when a source path is
/// available.
#[must_use]
pub fn merged_toml_settings(
    namespace: &str,
    embedded_toml: &str,
    embedded_source_path: &str,
    orphan_file: &str,
) -> Value {
    merged_toml_settings_with_override(
        namespace,
        embedded_toml,
        embedded_source_path,
        orphan_file,
        wendao_config_file_override(),
    )
}

/// Merge runtime TOML settings with an explicit user override path.
#[must_use]
pub fn merged_toml_settings_with_override(
    namespace: &str,
    embedded_toml: &str,
    embedded_source_path: &str,
    orphan_file: &str,
    user_override: Option<PathBuf>,
) -> Value {
    let spec = ConfigCascadeSpec::new(namespace, embedded_toml, orphan_file)
        .with_embedded_source_path(embedded_source_path);

    if let Some(user_path) = user_override {
        if let Ok(user_config) = load_toml_value_with_imports(user_path.as_path()) {
            let mut merged = load_embedded_defaults(embedded_toml, embedded_source_path);
            deep_merge(&mut merged, toml_value_to_yaml(&user_config));
            return merged;
        }
    }

    match resolve_and_merge_toml(spec) {
        Ok(toml_value) => {
            let json_str = serde_json::to_string(&toml_value)
                .ok()
                .unwrap_or_else(|| "{}".to_string());
            serde_json::from_str::<Value>(&json_str)
                .unwrap_or_else(|_| load_embedded_defaults(embedded_toml, embedded_source_path))
        }
        Err(_) => load_embedded_defaults(embedded_toml, embedded_source_path),
    }
}

fn load_embedded_defaults(embedded_toml: &str, embedded_source_path: &str) -> Value {
    let source_path = Path::new(embedded_source_path);
    if let Ok(toml_value) = load_toml_value_with_imports(source_path) {
        return toml_value_to_yaml(&toml_value);
    }

    let toml_value: toml::Value =
        toml::from_str(embedded_toml).unwrap_or(toml::Value::Table(toml::map::Map::new()));
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

#[cfg(test)]
mod tests {
    use super::merged_toml_settings_with_override;
    use crate::settings::get_setting_string;
    use serde_yaml::Value;
    use std::fs;

    fn yaml_value_at<'a>(value: &'a Value, dotted_key: &str) -> Option<&'a Value> {
        let mut cursor = value;
        for segment in dotted_key.split('.') {
            let mapping = cursor.as_mapping()?;
            let key = Value::String(segment.to_string());
            cursor = mapping.get(&key)?;
        }
        Some(cursor)
    }

    #[test]
    fn merged_toml_settings_with_override_prefers_user_config() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let user_path = temp.path().join("wendao.toml");
        fs::write(
            &user_path,
            r#"[link_graph.retrieval]
mode = "user"
"#,
        )
        .unwrap_or_else(|error| panic!("write config: {error}"));

        let settings = merged_toml_settings_with_override(
            "link_graph",
            "[link_graph.retrieval]\nmode = \"embedded\"\n",
            "/nonexistent/embedded-wendao.toml",
            "wendao.toml",
            Some(user_path),
        );

        assert_eq!(
            get_setting_string(&settings, "link_graph.retrieval.mode"),
            Some("user".to_string())
        );
    }

    #[test]
    fn merged_toml_settings_with_override_resolves_imports() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let shared_path = temp.path().join("shared.toml");
        let user_path = temp.path().join("wendao.toml");

        fs::write(
            &shared_path,
            r#"[link_graph.retrieval]
candidate_multiplier = 4
"#,
        )
        .unwrap_or_else(|error| panic!("write shared config: {error}"));
        fs::write(
            &user_path,
            r#"
imports = ["shared.toml"]

[link_graph.retrieval]
mode = "user"
"#,
        )
        .unwrap_or_else(|error| panic!("write user config: {error}"));

        let settings = merged_toml_settings_with_override(
            "link_graph",
            r#"[link_graph.retrieval]
mode = "embedded"
"#,
            "/nonexistent/embedded-wendao.toml",
            "wendao.toml",
            Some(user_path),
        );

        assert_eq!(
            get_setting_string(&settings, "link_graph.retrieval.mode"),
            Some("user".to_string())
        );
        let candidate_multiplier =
            yaml_value_at(&settings, "link_graph.retrieval.candidate_multiplier")
                .cloned()
                .and_then(|value| serde_yaml::from_value::<i64>(value).ok());
        assert_eq!(candidate_multiplier, Some(4));
    }
}
