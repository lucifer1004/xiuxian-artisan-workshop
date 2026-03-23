use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepositoryPluginConfig, RepositoryRefreshPolicy,
};
use crate::gateway::studio::repo_index::types::{RepoIndexEntryStatus, RepoIndexPhase};

use super::collect::{await_analysis_completion, collect_code_documents};
use super::coordinator::RepoIndexCoordinator;
use super::fingerprint::{fingerprint, timestamp_now};
use super::task::{AdaptiveConcurrencyController, RepoIndexTaskPriority};

fn repo(id: &str, path: &str) -> RegisteredRepository {
    RegisteredRepository {
        id: id.to_string(),
        path: Some(PathBuf::from(path)),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    }
}

#[test]
fn sync_repositories_only_enqueues_new_or_changed_repositories() {
    let coordinator = RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        crate::search_plane::SearchPlaneService::new(PathBuf::from(".")),
    );

    let first = coordinator.sync_repositories(vec![repo("sciml", "./sciml")]);
    let second = coordinator.sync_repositories(vec![repo("sciml", "./sciml")]);
    let third = coordinator.sync_repositories(vec![repo("sciml", "./sciml-next")]);

    assert_eq!(first, vec!["sciml".to_string()]);
    assert!(second.is_empty());
    assert_eq!(third, vec!["sciml".to_string()]);
}

#[test]
fn status_response_counts_each_phase() {
    let coordinator = RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        crate::search_plane::SearchPlaneService::new(PathBuf::from(".")),
    );
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "queued".to_string(),
        phase: RepoIndexPhase::Queued,
        queue_position: None,
        last_error: None,
        last_revision: None,
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "ready".to_string(),
        phase: RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: None,
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });
    coordinator.set_concurrency_for_test(AdaptiveConcurrencyController::new_for_test(6));

    let status = coordinator.status_response(None);
    assert_eq!(status.total, 2);
    assert_eq!(status.target_concurrency, 1);
    assert_eq!(status.max_concurrency, 6);
    assert_eq!(status.queued, 1);
    assert_eq!(status.ready, 1);
}

#[test]
fn status_response_filters_case_insensitively_from_cached_snapshot() {
    let coordinator = RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        crate::search_plane::SearchPlaneService::new(PathBuf::from(".")),
    );
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "DifferentialEquations.jl".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some(timestamp_now()),
        attempt_count: 2,
    });
    coordinator.mark_active_for_test("DifferentialEquations.jl");

    let status = coordinator.status_response(Some("differentialequations.jl"));
    assert_eq!(status.total, 1);
    assert_eq!(status.active, 1);
    assert_eq!(status.indexing, 1);
    assert_eq!(
        status.current_repo_id.as_deref(),
        Some("DifferentialEquations.jl")
    );
}

#[tokio::test]
async fn await_analysis_completion_returns_timeout_error_for_stuck_analysis() {
    let task = tokio::task::spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(25));
        Ok(crate::analyzers::RepositoryAnalysisOutput::default())
    });

    let Err(error) = await_analysis_completion("stuck", task, Duration::from_millis(1)).await
    else {
        panic!("slow analysis should time out");
    };

    match error {
        RepoIntelligenceError::AnalysisFailed { message } => {
            assert!(message.contains("repo `stuck` indexing timed out"));
        }
        other => panic!("expected analysis timeout failure, got {other:?}"),
    }
}

#[test]
fn record_repo_status_advances_attempt_count_without_lock_reentrancy() {
    let coordinator = RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        crate::search_plane::SearchPlaneService::new(PathBuf::from(".")),
    );
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "ADTypes.jl".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some(timestamp_now()),
        attempt_count: 2,
    });

    coordinator.record_repo_status(
        "ADTypes.jl",
        RepoIndexPhase::Ready,
        Some("abc123".to_string()),
        None,
    );

    let status = coordinator.status_response(Some("ADTypes.jl"));
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos.first().map(|item| item.attempt_count), Some(3));
}

#[test]
fn interactive_enqueue_promotes_pending_repository_to_front() {
    let coordinator = RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        crate::search_plane::SearchPlaneService::new(PathBuf::from(".")),
    );
    let first_repo = repo("ADTypes.jl", "./ADTypes.jl");
    let second_repo = repo("DifferentialEquations.jl", "./DifferentialEquations.jl");
    let first_fingerprint = fingerprint(&first_repo);
    let second_fingerprint = fingerprint(&second_repo);

    assert!(coordinator.enqueue_repository(
        first_repo,
        false,
        true,
        first_fingerprint,
        RepoIndexTaskPriority::Background,
    ));
    assert!(coordinator.enqueue_repository(
        second_repo.clone(),
        false,
        true,
        second_fingerprint.clone(),
        RepoIndexTaskPriority::Background,
    ));
    assert!(coordinator.enqueue_repository(
        second_repo,
        false,
        false,
        second_fingerprint,
        RepoIndexTaskPriority::Interactive,
    ));

    let pending = coordinator.pending_repo_ids_for_test();
    assert_eq!(
        pending,
        vec![
            "DifferentialEquations.jl".to_string(),
            "ADTypes.jl".to_string()
        ]
    );

    let status = coordinator.status_response(None);
    assert_eq!(
        status
            .repos
            .iter()
            .find(|repo| repo.repo_id == "DifferentialEquations.jl")
            .and_then(|repo| repo.queue_position),
        Some(1)
    );
    assert_eq!(
        status
            .repos
            .iter()
            .find(|repo| repo.repo_id == "ADTypes.jl")
            .and_then(|repo| repo.queue_position),
        Some(2)
    );
}

#[test]
fn adaptive_controller_expands_with_backlog_and_fast_feedback() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(4);

    assert_eq!(controller.target_limit(8, 0), 1);

    controller.record_success(Duration::from_millis(20), 7);
    assert_eq!(controller.target_limit(7, 0), 2);

    controller.record_success(Duration::from_millis(18), 6);
    assert_eq!(controller.target_limit(6, 0), 2);

    controller.record_success(Duration::from_millis(18), 5);
    assert_eq!(controller.target_limit(5, 0), 3);

    controller.record_failure();
    assert_eq!(controller.target_limit(5, 0), 1);
}

#[test]
fn adaptive_controller_contracts_when_efficiency_collapses() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(6);
    controller.current_limit = 4;
    controller.ema_elapsed_ms = Some(100.0);
    controller.baseline_elapsed_ms = Some(100.0);
    controller.previous_efficiency = Some(4.0 / 100.0);

    controller.record_success(Duration::from_millis(600), 8);

    assert_eq!(controller.target_limit(8, 0), 2);
}

#[test]
fn status_response_exposes_active_repos_and_concurrency_metadata() {
    let coordinator = RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        crate::search_plane::SearchPlaneService::new(PathBuf::from(".")),
    );
    coordinator.set_concurrency_for_test(AdaptiveConcurrencyController::new_for_test(8));
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "ADTypes.jl".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });
    coordinator.mark_active_for_test("ADTypes.jl");

    let status = coordinator.status_response(None);
    assert_eq!(status.active, 1);
    assert_eq!(status.current_repo_id.as_deref(), Some("ADTypes.jl"));
    assert_eq!(status.active_repo_ids, vec!["ADTypes.jl".to_string()]);
    assert_eq!(status.target_concurrency, 1);
    assert_eq!(status.max_concurrency, 8);
}

#[test]
fn collect_code_documents_returns_none_when_cancelled() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    std::fs::write(tempdir.path().join("module.jl"), "module Demo\nend\n")
        .unwrap_or_else(|error| panic!("write file: {error}"));

    let documents = collect_code_documents(tempdir.path(), || true);

    assert!(documents.is_none());
}
