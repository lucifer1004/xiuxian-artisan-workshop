use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;

use crate::analyzers::bootstrap_builtin_registry;
use crate::analyzers::{ModuleRecord, RelationRecord, RepoSymbolKind, SymbolRecord};
use crate::gateway::studio::router::handlers::graph::GraphNeighborsQuery;
use crate::gateway::studio::types::{
    UiConfig, UiProjectConfig, UiRepoProjectConfig, VfsScanResult,
};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::*;

fn studio_with_repo_projects(repo_projects: Vec<UiRepoProjectConfig>) -> StudioState {
    let studio = StudioState::new();
    studio.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects,
    });
    studio
}

fn repo_project(id: &str) -> UiRepoProjectConfig {
    UiRepoProjectConfig {
        id: id.to_string(),
        root: Some(".".to_string()),
        url: None,
        git_ref: None,
        refresh: None,
        plugins: vec!["julia".to_string()],
    }
}

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

#[tokio::test]
async fn graph_index_refreshes_after_document_title_changes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir).unwrap_or_else(|error| panic!("create docs dir: {error}"));
    fs::write(
        docs_dir.join("index.md"),
        concat!(
            "---\n",
            "title: Documentation Index\n",
            "---\n\n",
            "# Documentation Index\n\n",
            "Body.\n",
        ),
    )
    .unwrap_or_else(|error| panic!("write docs index: {error}"));
    fs::write(docs_dir.join("chapter.md"), "# Chapter\n\nBody.\n")
        .unwrap_or_else(|error| panic!("write docs chapter: {error}"));

    let mut studio = StudioState::new();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = temp_dir.path().to_path_buf();
    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });

    let first_response = graph_neighbors(
        State(Arc::clone(&state)),
        axum::extract::Path("kernel/docs/index.md".to_string()),
        axum::extract::Query(GraphNeighborsQuery {
            direction: Some("both".to_string()),
            hops: Some(1),
            limit: Some(20),
        }),
    )
    .await
    .unwrap_or_else(|error| panic!("initial graph neighbors should build: {error:?}"))
    .0;
    assert_eq!(first_response.center.label, "Documentation Index");

    fs::write(
        docs_dir.join("index.md"),
        concat!(
            "---\n",
            "title: Qianji Studio DocOS Kernel: Map of Content\n",
            "---\n\n",
            "# Qianji Studio DocOS Kernel: Map of Content\n\n",
            "- [[chapter]]\n",
        ),
    )
    .unwrap_or_else(|error| panic!("rewrite docs index: {error}"));

    let refreshed_response = graph_neighbors(
        State(Arc::clone(&state)),
        axum::extract::Path("kernel/docs/index.md".to_string()),
        axum::extract::Query(GraphNeighborsQuery {
            direction: Some("both".to_string()),
            hops: Some(1),
            limit: Some(20),
        }),
    )
    .await
    .unwrap_or_else(|error| panic!("refreshed graph neighbors should rebuild: {error:?}"))
    .0;
    assert_eq!(
        refreshed_response.center.label,
        "Qianji Studio DocOS Kernel: Map of Content"
    );
}

#[tokio::test]
async fn graph_neighbors_prefers_kernel_project_docs_over_repo_root_docs() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let config_root = temp_dir.path().join(".data/wendao-frontend");
    let kernel_docs_dir = config_root.join("docs");
    let main_docs_dir = temp_dir.path().join("docs");

    fs::create_dir_all(&kernel_docs_dir)
        .unwrap_or_else(|error| panic!("create kernel docs dir: {error}"));
    fs::create_dir_all(&main_docs_dir)
        .unwrap_or_else(|error| panic!("create main docs dir: {error}"));

    fs::write(
        kernel_docs_dir.join("index.md"),
        concat!(
            "---\n",
            "title: Qianji Studio DocOS Kernel: Map of Content\n",
            "---\n\n",
            "# Qianji Studio DocOS Kernel: Map of Content\n\n",
            "- [[chapter]]\n",
        ),
    )
    .unwrap_or_else(|error| panic!("write kernel docs index: {error}"));
    fs::write(
        kernel_docs_dir.join("chapter.md"),
        "# Kernel Chapter\n\nBody.\n",
    )
    .unwrap_or_else(|error| panic!("write kernel chapter: {error}"));
    fs::write(
        main_docs_dir.join("index.md"),
        concat!(
            "---\n",
            "title: Documentation Index\n",
            "---\n\n",
            "# Documentation Index\n\n",
            "Body.\n",
        ),
    )
    .unwrap_or_else(|error| panic!("write main docs index: {error}"));

    let mut studio = StudioState::new();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = config_root.clone();
    studio.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "main".to_string(),
                root: temp_dir.path().to_string_lossy().to_string(),
                dirs: vec!["docs".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });

    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });

    let kernel_response = graph_neighbors(
        State(Arc::clone(&state)),
        axum::extract::Path("kernel/docs/index.md".to_string()),
        axum::extract::Query(GraphNeighborsQuery {
            direction: Some("both".to_string()),
            hops: Some(1),
            limit: Some(20),
        }),
    )
    .await
    .unwrap_or_else(|error| panic!("kernel graph neighbors should resolve: {error:?}"))
    .0;
    assert_eq!(
        kernel_response.center.label,
        "Qianji Studio DocOS Kernel: Map of Content"
    );
    assert!(
        kernel_response
            .nodes
            .iter()
            .any(|node| node.id == "kernel/docs/chapter.md")
    );

    let main_response = graph_neighbors(
        State(Arc::clone(&state)),
        axum::extract::Path("main/docs/index.md".to_string()),
        axum::extract::Query(GraphNeighborsQuery {
            direction: Some("both".to_string()),
            hops: Some(1),
            limit: Some(20),
        }),
    )
    .await
    .unwrap_or_else(|error| panic!("main graph neighbors should resolve: {error:?}"))
    .0;
    assert_eq!(main_response.center.label, "Documentation Index");
    assert!(
        main_response
            .nodes
            .iter()
            .any(|node| node.id == "main/docs/index.md")
    );
}

#[test]
fn resolve_code_ast_repository_and_path_infers_repo_from_prefixed_path() {
    use super::code_ast::resolve_code_ast_repository_and_path;

    let studio = studio_with_repo_projects(vec![repo_project("sciml"), repo_project("mcl")]);
    let repositories = configured_repositories(&studio);
    let (repository, path) =
        resolve_code_ast_repository_and_path(&repositories, None, "sciml/src/BaseModelica.jl")
            .unwrap_or_else(|error| {
                panic!("repo should be inferred from prefixed path: {error:?}")
            });
    assert_eq!(repository.id, "sciml");
    assert_eq!(path, "src/BaseModelica.jl");
}

#[test]
fn resolve_code_ast_repository_and_path_requires_repo_when_ambiguous() {
    use super::code_ast::resolve_code_ast_repository_and_path;

    let studio = studio_with_repo_projects(vec![repo_project("sciml"), repo_project("mcl")]);
    let repositories = configured_repositories(&studio);
    let Err(error) =
        resolve_code_ast_repository_and_path(&repositories, None, "src/BaseModelica.jl")
    else {
        panic!("should fail when repo cannot be inferred");
    };
    assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_REPO");
}

#[test]
fn configured_repository_matches_repo_identifier_case_insensitively() {
    let studio = studio_with_repo_projects(vec![repo_project("DifferentialEquations.jl")]);

    let repository = configured_repository(&studio, "differentialequations.jl")
        .unwrap_or_else(|error| panic!("repo lookup should ignore ASCII case: {error:?}"));

    assert_eq!(repository.id, "DifferentialEquations.jl");
}

#[test]
fn build_code_ast_analysis_response_emits_uses_projection_and_external_node() {
    use crate::gateway::studio::types::{CodeAstEdgeKind, CodeAstNodeKind, CodeAstProjectionKind};

    let analysis = crate::analyzers::RepositoryAnalysisOutput {
        modules: vec![ModuleRecord {
            repo_id: "sciml".to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![
            SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:reexport".to_string(),
                module_id: Some("module:BaseModelica".to_string()),
                name: "reexport".to_string(),
                qualified_name: "BaseModelica.reexport".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/BaseModelica.jl".to_string(),
                line_start: Some(7),
                line_end: Some(9),
                signature: None,
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
            SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:ModelicaSystem".to_string(),
                module_id: None,
                name: "ModelicaSystem".to_string(),
                qualified_name: "ModelicaSystem".to_string(),
                kind: RepoSymbolKind::Type,
                path: "src/modelica/system.jl".to_string(),
                line_start: Some(1),
                line_end: Some(3),
                signature: None,
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
        ],
        relations: vec![RelationRecord {
            repo_id: "sciml".to_string(),
            source_id: "symbol:reexport".to_string(),
            target_id: "symbol:ModelicaSystem".to_string(),
            kind: crate::analyzers::RelationKind::Uses,
        }],
        ..crate::analyzers::RepositoryAnalysisOutput::default()
    };
    let payload = build_code_ast_analysis_response(
        "sciml".to_string(),
        "src/BaseModelica.jl".to_string(),
        Some(7),
        &analysis,
    );
    assert_eq!(payload.language, "julia");
    assert!(
        payload
            .nodes
            .iter()
            .any(|node| matches!(node.kind, CodeAstNodeKind::ExternalSymbol))
    );
    assert!(
        payload
            .edges
            .iter()
            .any(|edge| matches!(edge.kind, CodeAstEdgeKind::Uses))
    );
    assert!(payload.projections.iter().any(|projection| {
        matches!(projection.kind, CodeAstProjectionKind::Calls) && projection.edge_count > 0
    }));
    assert!(payload.focus_node_id.is_some());
}
