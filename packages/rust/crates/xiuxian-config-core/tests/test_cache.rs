use std::path::{Path, PathBuf};

use tempfile::TempDir;
use xiuxian_config_core::{ConfigCascadeSpec, resolve_and_merge_toml_with_paths};

pub(super) fn write_text(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create parent {}: {error}", parent.display()));
    }
    std::fs::write(path, content)
        .unwrap_or_else(|error| panic!("write fixture {}: {error}", path.display()));
}

pub(super) fn temp_workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("create temp workspace: {error}"));
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join(".config/xiuxian-artisan-workshop"))
        .unwrap_or_else(|error| panic!("create .config/xiuxian-artisan-workshop: {error}"));
    (temp, root)
}

pub(super) fn strict_mode_from_merged(value: &toml::Value) -> Option<bool> {
    value
        .get("validation")
        .and_then(|node| node.get("strict_mode"))
        .and_then(toml::Value::as_bool)
}

pub(super) fn skills_spec() -> ConfigCascadeSpec<'static> {
    ConfigCascadeSpec::new(
        "skills",
        r"
[validation]
strict_mode = true
",
        "skills.toml",
    )
}

pub(super) fn resolve_skills(root: &Path, spec: ConfigCascadeSpec) -> toml::Value {
    resolve_and_merge_toml_with_paths(spec, Some(root), Some(root.join(".config").as_path()))
        .unwrap_or_else(|error| panic!("resolve cached config: {error}"))
}
