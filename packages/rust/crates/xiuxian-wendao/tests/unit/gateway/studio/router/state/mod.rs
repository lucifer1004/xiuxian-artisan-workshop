use crate::gateway::studio::router::state::helpers::graph_include_dirs;
use crate::gateway::studio::router::state::lifecycle::gateway_bootstrap_background_indexing_with_lookup;
use crate::gateway::studio::router::state::{StudioState, supported_code_kinds};
use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
use crate::search::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
    SearchPlaneService,
};
use std::sync::Arc;

#[test]
fn supported_code_kinds_contains_reference_and_doc() {
    let kinds = supported_code_kinds();
    assert!(kinds.iter().any(|kind| kind == "reference"));
    assert!(kinds.iter().any(|kind| kind == "doc"));
}

#[test]
fn graph_include_dirs_deduplicates_normalized_paths() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path().to_path_buf();
    let config_root = temp_dir.path().to_path_buf();
    std::fs::create_dir_all(temp_dir.path().join("docs"))
        .unwrap_or_else(|error| panic!("create docs dir: {error}"));
    std::fs::create_dir_all(temp_dir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src dir: {error}"));

    let projects = vec![UiProjectConfig {
        name: "kernel".to_string(),
        root: ".".to_string(),
        dirs: vec![
            "docs".to_string(),
            "./docs".to_string(),
            "src".to_string(),
            "src/".to_string(),
        ],
    }];

    let include_dirs = graph_include_dirs(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
    );

    assert_eq!(include_dirs, vec!["docs".to_string(), "src".to_string()]);
}

#[test]
fn gateway_bootstrap_background_indexing_defaults_to_disabled() {
    assert!(!gateway_bootstrap_background_indexing_with_lookup(&|_| {
        None
    }));
    assert!(!gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("invalid".to_string())
    }));
    assert!(!gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("false".to_string())
    }));
}

#[test]
fn gateway_bootstrap_background_indexing_accepts_truthy_env_values() {
    assert!(gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("true".to_string())
    }));
    assert!(gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some(" YES ".to_string())
    }));
    assert!(gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("1".to_string())
    }));
}

#[test]
fn bootstrap_background_indexing_telemetry_reports_default_deferred_state() {
    let studio = StudioState::new();
    let telemetry = studio.bootstrap_background_indexing_telemetry();

    assert!(!telemetry.enabled());
    assert_eq!(telemetry.mode(), "deferred");
    assert!(!telemetry.deferred_activation_observed());
    assert_eq!(telemetry.deferred_activation_at(), None);
    assert_eq!(telemetry.deferred_activation_source(), None);
}

#[tokio::test]
async fn set_ui_config_starts_local_search_plane_indexes_for_configured_projects() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path().join("workspace");
    let storage_root = temp_dir.path().join("search_plane");
    std::fs::create_dir_all(project_root.join("docs"))
        .unwrap_or_else(|error| panic!("create docs dir: {error}"));
    std::fs::create_dir_all(project_root.join("src"))
        .unwrap_or_else(|error| panic!("create src dir: {error}"));
    std::fs::write(
        project_root.join("docs/intro.md"),
        "# Studio Search\n\nWarm local corpora before the first query.\n",
    )
    .unwrap_or_else(|error| panic!("write note: {error}"));
    std::fs::write(project_root.join("src/lib.rs"), "pub fn warmup() {}\n")
        .unwrap_or_else(|error| panic!("write source: {error}"));

    let plugin_registry = Arc::new(
        crate::analyzers::bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
    );
    let search_plane = SearchPlaneService::with_paths(
        project_root.clone(),
        storage_root,
        SearchManifestKeyspace::new("xiuxian:test:studio-state:background-local-corpora"),
        SearchMaintenancePolicy::default(),
    );
    let studio = StudioState::new_with_bootstrap_ui_config_for_roots_and_search_plane(
        plugin_registry,
        project_root.clone(),
        project_root.clone(),
        search_plane,
    );

    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string(), "src".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    wait_for_local_corpus_ready(&studio, SearchCorpusKind::KnowledgeSection).await;
    wait_for_local_corpus_ready(&studio, SearchCorpusKind::LocalSymbol).await;
    wait_for_local_corpus_ready(&studio, SearchCorpusKind::Attachment).await;
    wait_for_local_corpus_ready(&studio, SearchCorpusKind::ReferenceOccurrence).await;

    let telemetry = studio.bootstrap_background_indexing_telemetry();
    assert!(telemetry.deferred_activation_observed());
    assert_eq!(
        telemetry.deferred_activation_source(),
        Some("set_ui_config")
    );
}

async fn wait_for_local_corpus_ready(studio: &StudioState, corpus: SearchCorpusKind) {
    for _ in 0..200 {
        let status = studio.search_plane.coordinator().status_for(corpus);
        if status.phase == SearchPlanePhase::Ready && status.active_epoch.is_some() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("search corpus `{corpus}` did not reach ready state");
}
