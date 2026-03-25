use std::future::pending;
use std::path::PathBuf;

use super::super::types::SearchPlaneService;
use super::helpers::REPO_MAINTENANCE_SHUTDOWN_MESSAGE;
use crate::search_plane::coordinator::SearchCompactionReason;
use crate::search_plane::service::core::RepoMaintenanceTaskKind;
use crate::search_plane::{SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace};
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};
use xiuxian_vector::VectorStoreError;

#[test]
fn repo_maintenance_task_key_tracks_kind() {
    let prewarm =
        super::super::types::RepoMaintenanceTask::Prewarm(super::super::types::RepoPrewarmTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            projected_columns: vec!["path".to_string()],
        });
    let compaction = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-1".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            row_count: 12,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    assert_eq!(prewarm.task_key().3, RepoMaintenanceTaskKind::Prewarm);
    assert_eq!(compaction.task_key().3, RepoMaintenanceTaskKind::Compaction);
    assert_ne!(prewarm.task_key(), compaction.task_key());
}

#[test]
fn register_repo_maintenance_task_prioritizes_prewarm_before_compaction() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-priority"),
        SearchMaintenancePolicy::default(),
    );
    let compaction = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-1".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            row_count: 12,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    let prewarm =
        super::super::types::RepoMaintenanceTask::Prewarm(super::super::types::RepoPrewarmTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "beta/repo".to_string(),
            table_name: "repo_entity_beta".to_string(),
            projected_columns: vec!["path".to_string()],
        });

    let (_, compaction_enqueued, start_compaction_worker) =
        service.register_repo_maintenance_task(compaction.clone(), false);
    let (_, prewarm_enqueued, start_prewarm_worker) =
        service.register_repo_maintenance_task(prewarm.clone(), false);

    assert!(compaction_enqueued);
    assert!(prewarm_enqueued);
    assert!(start_compaction_worker);
    assert!(!start_prewarm_worker);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 2);
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Prewarm(_))
    ));
    assert!(matches!(
        runtime.queue.back().map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Compaction(_))
    ));
}

#[test]
fn register_repo_maintenance_task_replaces_queued_stale_compaction_for_same_repo() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-stale-compaction"),
        SearchMaintenancePolicy::default(),
    );
    let first = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-1".to_string(),
            table_name: "repo_entity_alpha_v1".to_string(),
            row_count: 12,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    let second = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-2".to_string(),
            table_name: "repo_entity_alpha_v2".to_string(),
            row_count: 8,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    let (_, first_enqueued, first_start_worker) =
        service.register_repo_maintenance_task(first.clone(), false);
    let (_, second_enqueued, second_start_worker) =
        service.register_repo_maintenance_task(second.clone(), false);

    assert!(first_enqueued);
    assert!(second_enqueued);
    assert!(first_start_worker);
    assert!(!second_start_worker);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 1);
    assert!(runtime.in_flight.contains(&second.task_key()));
    assert!(!runtime.in_flight.contains(&first.task_key()));
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Compaction(task))
            if task.publication_id == "publication-2"
    ));
}

#[test]
fn register_repo_maintenance_task_prioritizes_publish_threshold_before_row_delta_ratio() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-priority-reason"),
        SearchMaintenancePolicy::default(),
    );
    let row_delta = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoContentChunk,
            repo_id: "beta/repo".to_string(),
            publication_id: "publication-beta".to_string(),
            table_name: "repo_content_chunk_beta".to_string(),
            row_count: 8,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    let publish_threshold = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-alpha".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    let _ = service.register_repo_maintenance_task(row_delta, false);
    let _ = service.register_repo_maintenance_task(publish_threshold, false);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 2);
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "alpha/repo"
    ));
}

#[test]
fn register_repo_maintenance_task_prioritizes_smaller_row_count_within_same_reason() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-priority-row-count"),
        SearchMaintenancePolicy::default(),
    );
    let large = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoContentChunk,
            repo_id: "beta/repo".to_string(),
            publication_id: "publication-beta".to_string(),
            table_name: "repo_content_chunk_beta".to_string(),
            row_count: 64,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    let small = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-alpha".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            row_count: 8,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );

    let _ = service.register_repo_maintenance_task(large, false);
    let _ = service.register_repo_maintenance_task(small, false);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 2);
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "alpha/repo"
    ));
}

#[test]
fn register_repo_maintenance_task_ages_row_delta_ratio_ahead_of_new_publish_thresholds() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-aging"),
        SearchMaintenancePolicy::default(),
    );
    let aged_row_delta = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            publication_id: "publication-alpha".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            row_count: 16,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    let publish_one = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoContentChunk,
            repo_id: "beta/repo".to_string(),
            publication_id: "publication-beta".to_string(),
            table_name: "repo_content_chunk_beta".to_string(),
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    let publish_two = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "gamma/repo".to_string(),
            publication_id: "publication-gamma".to_string(),
            table_name: "repo_entity_gamma".to_string(),
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    let publish_three = super::super::types::RepoMaintenanceTask::Compaction(
        super::super::types::RepoCompactionTask {
            corpus: SearchCorpusKind::RepoContentChunk,
            repo_id: "delta/repo".to_string(),
            publication_id: "publication-delta".to_string(),
            table_name: "repo_content_chunk_delta".to_string(),
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    let _ = service.register_repo_maintenance_task(aged_row_delta, false);
    let _ = service.register_repo_maintenance_task(publish_one, false);
    let _ = service.register_repo_maintenance_task(publish_two, false);
    let _ = service.register_repo_maintenance_task(publish_three, false);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 4);
    assert!(matches!(
        runtime.queue.get(2).map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "alpha/repo"
    ));
    assert!(matches!(
        runtime.queue.get(3).map(|queued| &queued.task),
        Some(super::super::types::RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "delta/repo"
    ));
}

#[tokio::test]
async fn stop_repo_maintenance_clears_waiters_and_aborts_worker() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-stop"),
        SearchMaintenancePolicy::default(),
    );
    let task =
        super::super::types::RepoMaintenanceTask::Prewarm(super::super::types::RepoPrewarmTask {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            table_name: "repo_entity_alpha".to_string(),
            projected_columns: vec!["path".to_string()],
        });
    let task_key = task.task_key();
    let (sender, receiver) = oneshot::channel();
    let worker_handle = tokio::spawn(async {
        pending::<()>().await;
    });
    {
        let mut runtime = service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.in_flight.insert(task_key.clone());
        runtime.waiters.insert(task_key.clone(), vec![sender]);
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
                task,
                enqueue_sequence: 0,
            });
        runtime.worker_running = true;
        runtime.worker_handle = Some(worker_handle);
        runtime.active_task = Some(task_key.clone());
    }

    service.stop_repo_maintenance();

    let waiter_result = timeout(Duration::from_secs(1), receiver)
        .await
        .unwrap_or_else(|error| panic!("waiter timeout: {error}"))
        .unwrap_or_else(|error| panic!("waiter canceled: {error}"));
    assert_eq!(
        waiter_result,
        Err(REPO_MAINTENANCE_SHUTDOWN_MESSAGE.to_string())
    );
    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(runtime.shutdown_requested);
    assert!(runtime.in_flight.is_empty());
    assert!(runtime.waiters.is_empty());
    assert!(runtime.queue.is_empty());
    assert!(!runtime.worker_running);
    assert!(runtime.worker_handle.is_none());
    assert!(runtime.active_task.is_none());
}

#[tokio::test]
async fn prewarm_repo_table_rejects_new_tasks_after_shutdown() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-maintenance-shutdown-rejects"),
        SearchMaintenancePolicy::default(),
    );

    service.stop_repo_maintenance();

    let error = service
        .prewarm_repo_table(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            "repo_entity_alpha",
            &["path"],
        )
        .await
        .expect_err("shutdown should reject repo maintenance prewarm");
    assert!(matches!(
        error,
        VectorStoreError::General(message) if message == REPO_MAINTENANCE_SHUTDOWN_MESSAGE
    ));
}
