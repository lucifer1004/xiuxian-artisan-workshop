use std::sync::Arc;

use axum::extract::State;

use crate::analyzers::bootstrap_builtin_registry;
use crate::gateway::studio::router::tests::repo_project;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::types::{UiConfig, UiProjectConfig, VfsScanResult};
use crate::unified_symbol::UnifiedSymbolIndex;

#[test]
fn set_ui_config_preserves_cached_state_when_effectively_unchanged() {
    let studio = StudioState::new();
    let config = UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: vec![repo_project("sciml")],
    };
    studio.set_ui_config(config.clone());

    *studio
        .symbol_index
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(Arc::new(UnifiedSymbolIndex::new()));
    *studio
        .vfs_scan
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(VfsScanResult {
        entries: Vec::new(),
        file_count: 0,
        dir_count: 0,
        scan_duration_ms: 0,
    });

    studio.set_ui_config(config);

    assert!(
        studio
            .symbol_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    );
    assert!(
        studio
            .vfs_scan
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    );
}

#[tokio::test]
async fn ui_capabilities_reports_builtin_plugin_languages() {
    let registry = bootstrap_builtin_registry()
        .unwrap_or_else(|error| panic!("builtin registry should bootstrap: {error:?}"));
    let expected = registry
        .plugin_ids()
        .into_iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    let studio = StudioState::new_with_bootstrap_ui_config(Arc::new(registry));
    studio.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects: vec![repo_project("kernel"), repo_project("sciml")],
    });
    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });

    let response =
        crate::gateway::studio::router::handlers::get_ui_capabilities(State(Arc::clone(&state)))
            .await
            .unwrap_or_else(|error| panic!("ui capabilities should resolve: {error:?}"))
            .0;

    assert_eq!(response.languages, expected);
    assert_eq!(response.repositories, vec!["kernel", "sciml"]);
    assert_eq!(
        response.kinds,
        crate::gateway::studio::router::state::supported_code_kinds()
    );
}
