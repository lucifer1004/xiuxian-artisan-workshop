//! Shared helpers and a smoke test for config-core cache behavior.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use xiuxian_config_core::{ConfigCascadeSpec, resolve_and_merge_toml_with_paths};

fn write_text(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create parent {}: {error}", parent.display()));
    }
    std::fs::write(path, content)
        .unwrap_or_else(|error| panic!("write fixture {}: {error}", path.display()));
}

fn temp_workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("create temp workspace: {error}"));
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join(".config/xiuxian-artisan-workshop"))
        .unwrap_or_else(|error| panic!("create .config/xiuxian-artisan-workshop: {error}"));
    (temp, root)
}

fn strict_mode_from_merged(value: &toml::Value) -> Option<bool> {
    value
        .get("validation")
        .and_then(|node| node.get("strict_mode"))
        .and_then(toml::Value::as_bool)
}

fn skills_spec() -> ConfigCascadeSpec<'static> {
    ConfigCascadeSpec::new(
        "skills",
        r"
[validation]
strict_mode = true
",
        "skills.toml",
    )
}

fn resolve_skills(root: &Path, spec: ConfigCascadeSpec) -> toml::Value {
    resolve_and_merge_toml_with_paths(spec, Some(root), Some(root.join(".config").as_path()))
        .unwrap_or_else(|error| panic!("resolve cached config: {error}"))
}

#[test]
fn resolve_skills_applies_user_overlay() {
    let (_temp, root) = temp_workspace();
    write_text(
        root.join(".config/xiuxian-artisan-workshop/skills.toml")
            .as_path(),
        r#"
[validation]
strict_mode = false
"#,
    );

    let merged = resolve_skills(&root, skills_spec());
    assert_eq!(strict_mode_from_merged(&merged), Some(false));
}
