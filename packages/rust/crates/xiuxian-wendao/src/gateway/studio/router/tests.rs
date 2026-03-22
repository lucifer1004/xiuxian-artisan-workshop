use std::collections::BTreeMap;
use std::sync::Arc;

use axum::http::StatusCode;

use crate::analyzers::{ModuleRecord, RelationRecord, RepoSymbolKind, SymbolRecord};
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
