use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
use crate::gateway::studio::vfs::navigation::resolve_navigation_target;

#[test]
fn resolve_navigation_target_prefixes_configured_project_for_relative_docs_path() {
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

    let target = resolve_navigation_target(&state, "docs/index.md");

    assert_eq!(target.path, "main/docs/index.md");
    assert_eq!(target.project_name.as_deref(), Some("main"));
}
