//! Path-resolution helper regression tests.

use std::path::Path;

use xiuxian_config_core::test_support::resolve_home_from_value;

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
fn resolve_config_home_uses_project_default_when_env_missing() {
    let root = Path::new("/repo/project");
    let resolved = resolve_home_from_value(Some(root), None, ".config");
    assert_eq!(
        resolved.as_deref(),
        Some(Path::new("/repo/project/.config"))
    );
}
