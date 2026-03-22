use std::fs;
use std::sync::Arc;

use super::code_search::{
    build_code_search_response, build_repo_content_search_hits, parse_code_search_query,
    symbol_search_hit_to_search_hit,
};
use super::*;

#[test]
fn parse_code_search_query_extracts_repo_lang_and_kind_filters() {
    let parsed = parse_code_search_query("repo:sciml lang:julia kind:function reexport", None);
    assert_eq!(parsed.query, "reexport");
    assert_eq!(parsed.repo.as_deref(), Some("sciml"));
    assert_eq!(parsed.languages, vec!["julia".to_string()]);
    assert_eq!(parsed.kinds, vec!["function".to_string()]);
}

#[test]
fn parse_code_search_query_preserves_repo_identifier_case() {
    let parsed = parse_code_search_query(
        "repo:DifferentialEquations.jl using ModelingToolkit",
        Some("SciMLBase.jl"),
    );

    assert_eq!(parsed.query, "using ModelingToolkit");
    assert_eq!(parsed.repo.as_deref(), Some("DifferentialEquations.jl"));
}

#[test]
fn search_query_deserializes_query_alias() {
    let query: SearchQuery = serde_json::from_value(serde_json::json!({
        "query": "reexport",
        "intent": "code_search",
    }))
    .unwrap_or_else(|error| panic!("query alias should deserialize: {error}"));

    assert_eq!(query.q.as_deref(), Some("reexport"));
    assert_eq!(query.intent.as_deref(), Some("code_search"));
}

#[test]
fn symbol_search_hit_to_search_hit_preserves_backend_metadata() {
    let hit = symbol_search_hit_to_search_hit(
        "sciml",
        crate::analyzers::SymbolSearchHit {
            symbol: crate::analyzers::SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:reexport".to_string(),
                module_id: Some("module:BaseModelica".to_string()),
                name: "reexport".to_string(),
                qualified_name: "BaseModelica.reexport".to_string(),
                kind: crate::analyzers::RepoSymbolKind::Function,
                path: "src/BaseModelica.jl".to_string(),
                line_start: Some(7),
                line_end: Some(9),
                signature: Some("reexport()".to_string()),
                audit_status: Some("verified".to_string()),
                verification_state: Some("verified".to_string()),
                attributes: std::collections::BTreeMap::new(),
            },
            score: Some(0.8),
            rank: Some(1),
            saliency_score: Some(0.9),
            hierarchical_uri: Some("repo://sciml/symbol/reexport".to_string()),
            hierarchy: Some(vec!["src".to_string(), "BaseModelica.jl".to_string()]),
            implicit_backlinks: Some(vec!["doc:readme".to_string()]),
            implicit_backlink_items: Some(vec![crate::analyzers::RepoBacklinkItem {
                id: "doc:readme".to_string(),
                title: Some("README".to_string()),
                path: Some("README.md".to_string()),
                kind: Some("documents".to_string()),
            }]),
            projection_page_ids: Some(vec!["projection:1".to_string()]),
            audit_status: Some("verified".to_string()),
            verification_state: Some("verified".to_string()),
        },
    );

    assert_eq!(hit.doc_type.as_deref(), Some("symbol"));
    assert!(hit.tags.iter().any(|tag| tag == "lang:julia"));
    assert!(hit.tags.iter().any(|tag| tag == "kind:function"));
    assert!((hit.score - 0.9).abs() < f64::EPSILON);
    assert_eq!(
        hit.navigation_target.and_then(|target| target.project_name),
        Some("sciml".to_string())
    );
    assert_eq!(hit.audit_status.as_deref(), Some("verified"));
}

#[test]
fn repo_content_search_hits_find_matching_julia_source_lines() {
    let snapshot = RepoIndexSnapshot {
        repo_id: "sciml".to_string(),
        analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
        code_documents: Arc::new(vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module BaseModelica\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
            ),
        }]),
    };

    let hits = build_repo_content_search_hits(&snapshot, "lang:julia reexport", 10);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].doc_type.as_deref(), Some("file"));
    assert_eq!(hits[0].path, "src/BaseModelica.jl");
    assert_eq!(hits[0].match_reason.as_deref(), Some("repo_content_search"));
    assert_eq!(
        hits[0]
            .navigation_target
            .as_ref()
            .and_then(|target| target.line),
        Some(3)
    );
}

#[test]
fn build_code_search_response_skips_unsupported_repositories_when_searching_all_repos() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let valid_repo = temp.path().join("ValidPkg");
    fs::create_dir_all(valid_repo.join("src"))
        .unwrap_or_else(|error| panic!("create valid src: {error}"));
    fs::write(
        valid_repo.join("Project.toml"),
        "name = \"ValidPkg\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
    )
    .unwrap_or_else(|error| panic!("write project: {error}"));
    fs::write(
        valid_repo.join("src").join("ValidPkg.jl"),
        "module ValidPkg\nusing ModelingToolkit\nend\n",
    )
    .unwrap_or_else(|error| panic!("write valid source: {error}"));

    let invalid_repo = temp.path().join("DiffEqApproxFun.jl");
    fs::create_dir_all(invalid_repo.join("src"))
        .unwrap_or_else(|error| panic!("create invalid src: {error}"));
    fs::write(
        invalid_repo.join("src").join("DiffEqApproxFun.jl"),
        "module DiffEqApproxFun\nusing ApproxFun\nend\n",
    )
    .unwrap_or_else(|error| panic!("write invalid source: {error}"));

    let studio =
        crate::gateway::studio::router::StudioState::new_with_bootstrap_ui_config(Arc::new(
            crate::analyzers::bootstrap_builtin_registry()
                .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
        ));
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![
            crate::gateway::studio::types::UiRepoProjectConfig {
                id: "valid".to_string(),
                root: Some(valid_repo.display().to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            },
            crate::gateway::studio::types::UiRepoProjectConfig {
                id: "invalid".to_string(),
                root: Some(invalid_repo.display().to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            },
        ],
    });
    studio
        .repo_index
        .set_snapshot_for_test(Arc::new(RepoIndexSnapshot {
            repo_id: "valid".to_string(),
            analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
            code_documents: Arc::new(vec![crate::gateway::studio::repo_index::RepoCodeDocument {
                path: "src/ValidPkg.jl".to_string(),
                language: Some("julia".to_string()),
                contents: Arc::<str>::from("module ValidPkg\nusing ModelingToolkit\nend\n"),
            }]),
        }));
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("abc123".to_string()),
            updated_at: Some("2026-03-21T00:00:00Z".to_string()),
            attempt_count: 1,
        },
    );
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "invalid".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Unsupported,
            queue_position: None,
            last_error: Some("missing Project.toml".to_string()),
            last_revision: None,
            updated_at: Some("2026-03-21T00:00:00Z".to_string()),
            attempt_count: 1,
        },
    );

    let response = build_code_search_response(&studio, "ValidPkg".to_string(), None, 10)
        .unwrap_or_else(|error| {
            panic!("all-repo code search should skip unsupported repositories: {error:?}")
        });

    assert_eq!(response.query, "ValidPkg");
    assert_eq!(response.selected_mode.as_deref(), Some("code_search"));
    assert!(response.partial);
    assert_eq!(response.skipped_repos, vec!["invalid".to_string()]);
    assert!(response.hits.iter().all(|hit| {
        hit.navigation_target
            .as_ref()
            .and_then(|target| target.project_name.as_deref())
            != Some("invalid")
    }));
}

#[test]
fn build_code_search_response_returns_pending_payload_for_explicit_repo_without_snapshot() {
    let studio =
        crate::gateway::studio::router::StudioState::new_with_bootstrap_ui_config(Arc::new(
            crate::analyzers::bootstrap_builtin_registry()
                .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
        ));
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: "DifferentialEquations.jl".to_string(),
            root: Some(".".to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "DifferentialEquations.jl".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Queued,
            queue_position: None,
            last_error: None,
            last_revision: None,
            updated_at: Some("2026-03-21T00:00:00Z".to_string()),
            attempt_count: 1,
        },
    );

    let response = build_code_search_response(
        &studio,
        "using ModelingToolkit".to_string(),
        Some("DifferentialEquations.jl"),
        5,
    )
    .unwrap_or_else(|error| panic!("repo-specific pending search should not block: {error:?}"));

    assert!(response.hits.is_empty());
    assert!(response.partial);
    assert_eq!(response.indexing_state.as_deref(), Some("indexing"));
    assert_eq!(
        response.pending_repos,
        vec!["DifferentialEquations.jl".to_string()]
    );
    assert!(response.skipped_repos.is_empty());
}
