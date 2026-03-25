use std::fs;
use std::sync::Arc;
use std::time::Duration;

use crate::gateway::studio::repo_index::{
    RepoCodeDocument, RepoIndexEntryStatus, RepoIndexPhase, RepoIndexSnapshot,
};
use crate::gateway::studio::search::handlers::code_search::search::{
    build_code_search_response, build_code_search_response_with_budget,
    build_repo_entity_search_hits,
};
use crate::gateway::studio::search::handlers::tests::{
    publish_repo_content_chunk_index, publish_repo_entity_index, sample_repo_analysis,
    test_studio_state,
};

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
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "valid".to_string(),
        phase: RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some("2026-03-22T00:00:00Z".to_string()),
        attempt_count: 1,
    });

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
async fn build_code_search_response_prefers_repo_entity_hits_before_repo_content_fallback() {
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
    publish_repo_content_chunk_index(
        &studio,
        "valid",
        vec![RepoCodeDocument {
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
    studio
        .repo_index
        .set_snapshot_for_test(&Arc::new(RepoIndexSnapshot {
            repo_id: "valid".to_string(),
            analysis: Arc::new(analysis),
        }));
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "valid".to_string(),
        phase: RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some("2026-03-22T00:00:00Z".to_string()),
        attempt_count: 1,
    });

    let response = build_code_search_response(&studio, "reexport".to_string(), None, 10)
        .await
        .unwrap_or_else(|error| panic!("repo-wide code search response: {error:?}"));

    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("symbol")
                && hit.path == "src/BaseModelica.jl"),
        "expected repo entity symbol hit in repo-wide search: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
    assert!(
        response
            .hits
            .iter()
            .all(|hit| hit.doc_type.as_deref() != Some("file")),
        "repo entity hit should suppress repo-content fallback for the same repo: {:?}",
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
        vec![RepoCodeDocument {
            path: "src/ValidPkg.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from("module ValidPkg\nusing ModelingToolkit\nend\n"),
            size_bytes: 40,
            modified_unix_ms: 0,
        }],
    )
    .await;
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "valid".to_string(),
        phase: RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some("2026-03-21T00:00:00Z".to_string()),
        attempt_count: 1,
    });
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "invalid".to_string(),
        phase: RepoIndexPhase::Unsupported,
        queue_position: None,
        last_error: Some("missing Project.toml".to_string()),
        last_revision: None,
        updated_at: Some("2026-03-21T00:00:00Z".to_string()),
        attempt_count: 1,
    });

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
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "DifferentialEquations.jl".to_string(),
        phase: RepoIndexPhase::Queued,
        queue_position: None,
        last_error: None,
        last_revision: None,
        updated_at: Some("2026-03-21T00:00:00Z".to_string()),
        attempt_count: 1,
    });

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
        vec![RepoCodeDocument {
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
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "valid".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("def456".to_string()),
        updated_at: Some("2026-03-23T00:00:00Z".to_string()),
        attempt_count: 2,
    });

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

#[tokio::test]
async fn build_code_search_response_marks_partial_when_repo_wide_budget_expires() {
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
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "valid".to_string(),
        phase: RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some("2026-03-25T00:00:00Z".to_string()),
        attempt_count: 1,
    });

    let permit_count = studio
        .search_plane
        .repo_search_read_permits
        .available_permits();
    let held = Arc::clone(&studio.search_plane.repo_search_read_permits)
        .acquire_many_owned(u32::try_from(permit_count).unwrap_or(u32::MAX))
        .await
        .unwrap_or_else(|error| panic!("hold repo search permits: {error}"));

    let response = build_code_search_response_with_budget(
        &studio,
        "reexport".to_string(),
        None,
        10,
        Some(Duration::from_millis(1)),
    )
    .await
    .unwrap_or_else(|error| panic!("repo-wide timeout should return partial response: {error:?}"));

    drop(held);

    assert!(response.partial);
    assert_eq!(response.indexing_state.as_deref(), Some("partial"));
    assert_eq!(response.hit_count, 0);
    assert!(response.pending_repos.is_empty());
    assert!(response.skipped_repos.is_empty());
}
