use std::fs;
use std::sync::Arc;

use super::code_search::{
    build_code_search_response, build_repo_content_search_hits, build_repo_entity_search_hits,
    collect_repo_search_targets, parse_code_search_query, symbol_search_hit_to_search_hit,
};
use super::knowledge::build_intent_search_response;
use super::test_prelude::*;
use crate::search_plane::{RepoSearchAvailability, RepoSearchPublicationState};

fn test_studio_state() -> crate::gateway::studio::router::StudioState {
    let nonce = format!(
        "search-plane-handlers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|error| panic!("system time before unix epoch: {error}"))
            .as_nanos()
    );
    let search_plane_root = std::env::temp_dir().join(nonce);
    crate::gateway::studio::router::StudioState::new_with_bootstrap_ui_config_and_search_plane_root(
        Arc::new(
            crate::analyzers::bootstrap_builtin_registry()
                .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
        ),
        search_plane_root,
    )
}

async fn publish_repo_content_chunk_index(
    studio: &crate::gateway::studio::router::StudioState,
    repo_id: &str,
    documents: Vec<crate::gateway::studio::repo_index::RepoCodeDocument>,
) {
    studio
        .search_plane
        .publish_repo_content_chunks_with_revision(repo_id, &documents, None)
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
}

async fn publish_repo_entity_index(
    studio: &crate::gateway::studio::router::StudioState,
    repo_id: &str,
    analysis: &crate::analyzers::RepositoryAnalysisOutput,
) {
    studio
        .search_plane
        .publish_repo_entities_with_revision(repo_id, analysis, &sample_repo_documents(), None)
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
}

fn sample_repo_analysis(repo_id: &str) -> crate::analyzers::RepositoryAnalysisOutput {
    crate::analyzers::RepositoryAnalysisOutput {
        modules: vec![crate::analyzers::ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![crate::analyzers::SymbolRecord {
            repo_id: repo_id.to_string(),
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
        }],
        examples: vec![crate::analyzers::ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: "example:reexport".to_string(),
            title: "Reexport example".to_string(),
            path: "examples/reexport.jl".to_string(),
            summary: Some("Shows how to reexport ModelingToolkit".to_string()),
        }],
        ..crate::analyzers::RepositoryAnalysisOutput::default()
    }
}

fn sample_repo_documents() -> Vec<crate::gateway::studio::repo_index::RepoCodeDocument> {
    vec![
        crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module BaseModelica\nexport reexport\nreexport() = nothing\nend\n",
            ),
            size_bytes: 61,
            modified_unix_ms: 10,
        },
        crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "examples/reexport.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from("using BaseModelica\nreexport()\n"),
            size_bytes: 29,
            modified_unix_ms: 10,
        },
    ]
}

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
fn collect_repo_search_targets_preserves_repo_order_and_partitions_by_availability() {
    let publication_states = std::collections::BTreeMap::from([
        (
            "ready".to_string(),
            RepoSearchPublicationState {
                entity_published: true,
                content_published: false,
                availability: RepoSearchAvailability::Searchable,
            },
        ),
        (
            "pending".to_string(),
            RepoSearchPublicationState {
                entity_published: false,
                content_published: false,
                availability: RepoSearchAvailability::Pending,
            },
        ),
        (
            "skipped".to_string(),
            RepoSearchPublicationState {
                entity_published: false,
                content_published: false,
                availability: RepoSearchAvailability::Skipped,
            },
        ),
    ]);

    let dispatch = collect_repo_search_targets(
        vec![
            "ready".to_string(),
            "pending".to_string(),
            "skipped".to_string(),
            "implicit-pending".to_string(),
        ],
        &publication_states,
    );

    assert_eq!(
        dispatch
            .searchable_repos
            .into_iter()
            .map(|target| target.repo_id)
            .collect::<Vec<_>>(),
        vec!["ready".to_string()]
    );
    assert_eq!(
        dispatch.pending_repos,
        vec!["pending".to_string(), "implicit-pending".to_string()]
    );
    assert_eq!(dispatch.skipped_repos, vec!["skipped".to_string()]);
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

#[tokio::test]
async fn repo_content_search_hits_find_matching_julia_source_lines() {
    let studio = test_studio_state();
    publish_repo_content_chunk_index(
        &studio,
        "sciml",
        vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module BaseModelica\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
            ),
            size_bytes: 67,
            modified_unix_ms: 0,
        }],
    )
    .await;

    let hits = build_repo_content_search_hits(&studio, "sciml", "lang:julia reexport", 10)
        .await
        .unwrap_or_else(|error| panic!("repo content search hits: {error:?}"));

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

#[tokio::test]
async fn repo_content_search_hits_find_matching_code_punctuation_queries() {
    let studio = test_studio_state();
    publish_repo_content_chunk_index(
        &studio,
        "sciml",
        vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module BaseModelica\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
            ),
            size_bytes: 67,
            modified_unix_ms: 0,
        }],
    )
    .await;

    let hits = build_repo_content_search_hits(&studio, "sciml", "@reexport", 10)
        .await
        .unwrap_or_else(|error| panic!("repo content punctuation search hits: {error:?}"));

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "src/BaseModelica.jl");
    assert_eq!(
        hits[0]
            .navigation_target
            .as_ref()
            .and_then(|target| target.line),
        Some(3)
    );
}

#[tokio::test]
async fn build_code_search_response_returns_repo_entity_hits_from_search_plane() {
    let studio = test_studio_state();
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: "valid".to_string(),
            root: Some(".".to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    let analysis = sample_repo_analysis("valid");
    publish_repo_entity_index(&studio, "valid", &analysis).await;
    studio
        .repo_index
        .set_snapshot_for_test(&Arc::new(RepoIndexSnapshot {
            repo_id: "valid".to_string(),
            analysis: Arc::new(analysis),
        }));
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("abc123".to_string()),
            updated_at: Some("2026-03-22T00:00:00Z".to_string()),
            attempt_count: 1,
        },
    );

    let direct_hits = build_repo_entity_search_hits(&studio, "valid", "reexport", 10)
        .await
        .unwrap_or_else(|error| panic!("direct repo entity search hits: {error:?}"));
    assert!(
        direct_hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("symbol")
                && hit.path == "src/BaseModelica.jl"),
        "expected direct repo entity symbol hit: {:?}",
        direct_hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );

    let response = build_code_search_response(&studio, "reexport".to_string(), Some("valid"), 10)
        .await
        .unwrap_or_else(|error| panic!("code search response: {error:?}"));

    assert_eq!(response.selected_mode.as_deref(), Some("code_search"));
    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("symbol")
                && hit.path == "src/BaseModelica.jl"),
        "expected repo entity hit in code search response: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn build_code_search_response_skips_unsupported_repositories_when_searching_all_repos() {
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

    let studio = test_studio_state();
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
        .set_snapshot_for_test(&Arc::new(RepoIndexSnapshot {
            repo_id: "valid".to_string(),
            analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
        }));
    publish_repo_content_chunk_index(
        &studio,
        "valid",
        vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/ValidPkg.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from("module ValidPkg\nusing ModelingToolkit\nend\n"),
            size_bytes: 40,
            modified_unix_ms: 0,
        }],
    )
    .await;
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
        .await
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

#[tokio::test]
async fn build_code_search_response_returns_pending_payload_for_explicit_repo_without_snapshot() {
    let studio = test_studio_state();
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
    .await
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

#[tokio::test]
async fn build_intent_search_response_includes_repo_content_hits_for_debug_lookup() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let valid_repo = temp.path().join("ValidPkg");
    fs::create_dir_all(valid_repo.join("src"))
        .unwrap_or_else(|error| panic!("create valid src: {error}"));
    fs::write(
        valid_repo.join("Project.toml"),
        "name = \"ValidPkg\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
    )
    .unwrap_or_else(|error| panic!("write project: {error}"));

    let studio = test_studio_state();
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: "valid".to_string(),
            root: Some(valid_repo.display().to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    let snapshot = Arc::new(RepoIndexSnapshot {
        repo_id: "valid".to_string(),
        analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
    });
    publish_repo_content_chunk_index(
        &studio,
        "valid",
        vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/ValidPkg.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module ValidPkg\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
            ),
            size_bytes: 62,
            modified_unix_ms: 0,
        }],
    )
    .await;
    studio.repo_index.set_snapshot_for_test(&snapshot);
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("abc123".to_string()),
            updated_at: Some("2026-03-22T00:00:00Z".to_string()),
            attempt_count: 1,
        },
    );
    let direct_hits = build_repo_content_search_hits(&studio, "valid", "lang:julia reexport", 10)
        .await
        .unwrap_or_else(|error| panic!("direct repo content search hits: {error:?}"));
    assert_eq!(direct_hits.len(), 1);
    assert_eq!(direct_hits[0].path, "src/ValidPkg.jl");

    let response = build_intent_search_response(
        &studio,
        "lang:julia reexport",
        "lang:julia reexport",
        Some("valid"),
        10,
        Some("debug_lookup".to_string()),
    )
    .await
    .unwrap_or_else(|error| panic!("intent search response: {error:?}"));

    assert_eq!(response.selected_mode.as_deref(), Some("intent_hybrid"));
    assert_eq!(response.graph_confidence_score, Some(0.0));
    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("file") && hit.path == "src/ValidPkg.jl"),
        "expected repo content hit in hybrid intent response: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn build_intent_search_response_includes_repo_entity_hits_for_debug_lookup() {
    let studio = test_studio_state();
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: "valid".to_string(),
            root: Some(".".to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    let analysis = sample_repo_analysis("valid");
    publish_repo_entity_index(&studio, "valid", &analysis).await;
    studio
        .repo_index
        .set_snapshot_for_test(&Arc::new(RepoIndexSnapshot {
            repo_id: "valid".to_string(),
            analysis: Arc::new(analysis),
        }));
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("abc123".to_string()),
            updated_at: Some("2026-03-22T00:00:00Z".to_string()),
            attempt_count: 1,
        },
    );

    let response = build_intent_search_response(
        &studio,
        "reexport",
        "reexport",
        Some("valid"),
        10,
        Some("debug_lookup".to_string()),
    )
    .await
    .unwrap_or_else(|error| panic!("intent search response: {error:?}"));

    assert_eq!(response.selected_mode.as_deref(), Some("intent_hybrid"));
    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("symbol")
                && hit.path == "src/BaseModelica.jl"),
        "expected repo entity hit in hybrid intent response: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn build_code_search_response_uses_published_repo_tables_while_repo_refreshes() {
    let studio = test_studio_state();
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: "valid".to_string(),
            root: Some(".".to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    publish_repo_entity_index(&studio, "valid", &sample_repo_analysis("valid")).await;
    publish_repo_content_chunk_index(
        &studio,
        "valid",
        vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module BaseModelica\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
            ),
            size_bytes: 67,
            modified_unix_ms: 0,
        }],
    )
    .await;
    studio.repo_index.set_status_for_test(
        crate::gateway::studio::repo_index::RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: crate::gateway::studio::repo_index::RepoIndexPhase::Indexing,
            queue_position: None,
            last_error: None,
            last_revision: Some("def456".to_string()),
            updated_at: Some("2026-03-23T00:00:00Z".to_string()),
            attempt_count: 2,
        },
    );

    let response = build_code_search_response(&studio, "reexport".to_string(), Some("valid"), 10)
        .await
        .unwrap_or_else(|error| {
            panic!("refreshing repo should still serve published hits: {error:?}")
        });

    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("symbol")
                && hit.path == "src/BaseModelica.jl"),
        "expected published repo entity hit while repo refreshes: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
    assert!(response.pending_repos.is_empty());
}
