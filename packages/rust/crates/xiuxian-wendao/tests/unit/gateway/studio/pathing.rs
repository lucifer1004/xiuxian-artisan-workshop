use std::path::Path;

use crate::gateway::studio::pathing::{resolve_path_like, studio_display_path};
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig};

#[test]
fn resolve_path_like_expands_tilde_prefixed_home_paths() {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return;
    };

    let resolved = resolve_path_like(Path::new("/tmp/studio"), "~/workspace/docs")
        .unwrap_or_else(|| panic!("tilde-prefixed path should resolve"));

    assert_eq!(resolved, home.join("workspace/docs"));
}

#[test]
fn resolve_path_like_keeps_relative_paths_rooted_at_base() {
    let resolved = resolve_path_like(Path::new("/tmp/studio"), "docs")
        .unwrap_or_else(|| panic!("relative path should resolve"));

    assert_eq!(resolved, std::path::PathBuf::from("/tmp/studio/docs"));
}

#[test]
fn resolve_path_like_normalizes_current_dir_segments() {
    let resolved = resolve_path_like(Path::new("/tmp/studio"), ".")
        .unwrap_or_else(|| panic!("current-dir path should resolve"));

    assert_eq!(resolved, std::path::PathBuf::from("/tmp/studio"));
}

#[test]
fn studio_display_path_prefixes_configured_project_for_relative_paths() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "main".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string(), "internal_skills".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    assert_eq!(
        studio_display_path(&state, "docs/index.md"),
        "main/docs/index.md"
    );
}

#[test]
fn studio_display_path_keeps_existing_project_prefixes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "main".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    assert_eq!(
        studio_display_path(&state, "main/docs/index.md"),
        "main/docs/index.md"
    );
}

#[test]
fn studio_display_path_strips_relative_project_root_prefixes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: "frontend".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    assert_eq!(
        studio_display_path(&state, "frontend/docs/index.md"),
        "kernel/docs/index.md"
    );
}

#[test]
fn studio_display_path_prefers_project_root_relative_prefix_for_kernel_docs() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().join(".data/wendao-frontend");
    state.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "main".to_string(),
                root: temp_dir.path().to_path_buf().to_string_lossy().to_string(),
                dirs: vec!["docs".to_string(), "internal_skills".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });

    assert_eq!(
        studio_display_path(&state, ".data/wendao-frontend/docs/index.md"),
        "kernel/docs/index.md"
    );
    assert_eq!(
        studio_display_path(&state, "docs/index.md"),
        "main/docs/index.md"
    );
}
