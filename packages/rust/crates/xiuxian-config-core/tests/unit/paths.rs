//! Path-resolution helper regression tests.

use std::path::Path;

use xiuxian_config_core::{
    resolve_cache_home_from_value, resolve_home_from_value, resolve_path_from_value,
    resolve_project_root_or_cwd_from_value,
};

#[test]
fn resolve_data_home_uses_project_default_when_env_missing() {
    let root = Path::new("/repo/project");
    let resolved = resolve_home_from_value(Some(root), None, ".data");
    assert_eq!(resolved.as_deref(), Some(Path::new("/repo/project/.data")));
}

#[test]
fn resolve_data_home_resolves_relative_env_against_project_root() {
    let root = Path::new("/repo/project");
    let resolved = resolve_home_from_value(Some(root), Some(".state/data"), ".data");
    assert_eq!(
        resolved.as_deref(),
        Some(Path::new("/repo/project/.state/data"))
    );
}

#[test]
fn resolve_cache_home_respects_absolute_env_value() {
    let root = Path::new("/repo/project");
    let resolved = resolve_home_from_value(Some(root), Some("/tmp/cache-root"), ".cache");
    assert_eq!(resolved.as_deref(), Some(Path::new("/tmp/cache-root")));
}

#[test]
fn resolve_cache_home_from_value_uses_project_default_when_env_missing() {
    let root = Path::new("/repo/project");
    let resolved = resolve_cache_home_from_value(Some(root), None);
    assert_eq!(resolved.as_deref(), Some(Path::new("/repo/project/.cache")));
}

#[test]
fn resolve_cache_home_from_value_resolves_relative_env_against_project_root() {
    let root = Path::new("/repo/project");
    let resolved = resolve_cache_home_from_value(Some(root), Some(".runtime/cache"));
    assert_eq!(
        resolved.as_deref(),
        Some(Path::new("/repo/project/.runtime/cache"))
    );
}

#[test]
fn resolve_config_home_uses_project_default_when_env_missing() {
    let root = Path::new("/repo/project");
    let resolved = resolve_home_from_value(Some(root), None, ".config");
    assert_eq!(
        resolved.as_deref(),
        Some(Path::new("/repo/project/.config"))
    );
}

#[test]
fn resolve_path_from_value_resolves_relative_against_project_root() {
    let root = Path::new("/repo/project");
    let resolved = resolve_path_from_value(Some(root), Some(" .cache/state "));
    assert_eq!(
        resolved.as_deref(),
        Some(Path::new("/repo/project/.cache/state"))
    );
}

#[test]
fn resolve_path_from_value_preserves_absolute_input() {
    let root = Path::new("/repo/project");
    let resolved = resolve_path_from_value(Some(root), Some(" /tmp/cache-root "));
    assert_eq!(resolved.as_deref(), Some(Path::new("/tmp/cache-root")));
}

#[test]
fn resolve_project_root_or_cwd_from_value_uses_relative_env_against_cwd() {
    let cwd = Path::new("/repo/project");
    let resolved = resolve_project_root_or_cwd_from_value(Some("workspace"), Some(cwd));
    assert_eq!(resolved, Path::new("/repo/project/workspace"));
}

#[test]
fn resolve_project_root_or_cwd_from_value_falls_back_to_cwd_when_env_is_missing() {
    let cwd = Path::new("/repo/project");
    let resolved = resolve_project_root_or_cwd_from_value(None, Some(cwd));
    assert_eq!(resolved, Path::new("/repo/project"));
}

#[test]
fn resolve_project_root_or_cwd_from_value_falls_back_to_dot_without_cwd() {
    let resolved = resolve_project_root_or_cwd_from_value(Some("   "), None);
    assert_eq!(resolved, Path::new("."));
}
