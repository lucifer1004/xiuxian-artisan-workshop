use std::path::PathBuf;

use super::super::types::SearchPlaneService;
use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search_plane::coordinator::{SearchCompactionReason, SearchCompactionTask};
use crate::search_plane::service::helpers::repo_corpus_staging_epoch;
use crate::search_plane::{
    SearchCorpusKind, SearchCorpusStatusAction, SearchCorpusStatusReasonCode,
    SearchCorpusStatusSeverity, SearchMaintenancePolicy, SearchMaintenanceStatus,
    SearchManifestKeyspace, SearchPlanePhase, SearchRepoCorpusRecord, SearchRepoPublicationInput,
    SearchRepoPublicationRecord, SearchRepoRuntimeRecord,
};

#[test]
fn synthesize_repo_status_marks_indexing_corpus_as_prewarming_when_staging_was_prewarmed() {
    let runtime_status = RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-2".to_string()),
        updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        attempt_count: 1,
    };
    let staging_epoch = repo_corpus_staging_epoch(
        SearchCorpusKind::RepoEntity,
        &[runtime_status.clone()],
        None,
    )
    .unwrap_or_else(|| panic!("staging epoch should exist"));
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord::from_status(&runtime_status)),
        None,
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        last_prewarmed_at: Some("2026-03-24T12:34:57Z".to_string()),
        last_prewarmed_epoch: Some(staging_epoch),
        ..SearchMaintenanceStatus::default()
    }));

    let status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);

    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert_eq!(status.staging_epoch, Some(staging_epoch));
    assert_eq!(status.maintenance.last_prewarmed_epoch, Some(staging_epoch));
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Prewarming);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(!reason.readable);
}

#[test]
fn synthesize_repo_status_marks_indexing_corpus_as_prewarming_when_prewarm_is_running() {
    let runtime_status = RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-2".to_string()),
        updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        attempt_count: 1,
    };
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord::from_status(&runtime_status)),
        None,
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        prewarm_running: true,
        ..SearchMaintenanceStatus::default()
    }));

    let status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);

    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert!(status.maintenance.prewarm_running);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Prewarming);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(!reason.readable);
}

#[test]
fn annotate_runtime_status_marks_repo_prewarm_running_from_active_task() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-prewarm-active"),
        SearchMaintenancePolicy::default(),
    );
    let runtime_status = RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-2".to_string()),
        updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        attempt_count: 1,
    };
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord::from_status(&runtime_status)),
        None,
    );
    service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_task = Some((
        SearchCorpusKind::RepoEntity,
        "alpha/repo".to_string(),
        "repo_entity_repo_alpha".to_string(),
        super::super::types::RepoMaintenanceTaskKind::Prewarm,
    ));

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(status.maintenance.prewarm_running);
    assert_eq!(status.maintenance.prewarm_queue_depth, 0);
    assert_eq!(status.maintenance.prewarm_queue_position, None);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Prewarming);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(!reason.readable);
}

#[test]
fn annotate_runtime_status_surfaces_repo_prewarm_queue_backlog() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-prewarm-queue"),
        SearchMaintenancePolicy::default(),
    );
    let runtime_status = RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-2".to_string()),
        updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        attempt_count: 1,
    };
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord::from_status(&runtime_status)),
        None,
    );
    {
        let mut runtime = service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
            task: super::super::types::RepoMaintenanceTask::Compaction(
                super::super::types::RepoCompactionTask {
                    corpus: SearchCorpusKind::RepoEntity,
                    repo_id: "beta/repo".to_string(),
                    publication_id: "publication-beta".to_string(),
                    table_name: "repo_entity_repo_beta".to_string(),
                    row_count: 12,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
            ),
            enqueue_sequence: 0,
        });
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
                task: super::super::types::RepoMaintenanceTask::Prewarm(
                    super::super::types::RepoPrewarmTask {
                        corpus: SearchCorpusKind::RepoEntity,
                        repo_id: "alpha/repo".to_string(),
                        table_name: "repo_entity_repo_alpha".to_string(),
                        projected_columns: vec!["name".to_string()],
                    },
                ),
                enqueue_sequence: 1,
            });
    }

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(!status.maintenance.prewarm_running);
    assert_eq!(status.maintenance.prewarm_queue_depth, 1);
    assert_eq!(status.maintenance.prewarm_queue_position, Some(2));
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::WarmingUp);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(!reason.readable);
}

#[test]
fn annotate_runtime_status_preserves_repo_compaction_running_from_record_maintenance() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_running: true,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(status.maintenance.compaction_running);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Compacting);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(reason.readable);
}

#[test]
fn annotate_runtime_status_marks_repo_compaction_running_from_active_task() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction-active"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_running: false,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));
    service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_task = Some((
        SearchCorpusKind::RepoEntity,
        "alpha/repo".to_string(),
        "publication-alpha".to_string(),
        super::super::types::RepoMaintenanceTaskKind::Compaction,
    ));

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 0);
    assert_eq!(status.maintenance.compaction_queue_position, None);
    assert!(!status.maintenance.compaction_queue_aged);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Compacting);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(reason.readable);
}

#[test]
fn annotate_runtime_status_surfaces_repo_compaction_queue_backlog() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction-queue"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));
    {
        let mut runtime = service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
                task: super::super::types::RepoMaintenanceTask::Prewarm(
                    super::super::types::RepoPrewarmTask {
                        corpus: SearchCorpusKind::RepoEntity,
                        repo_id: "beta/repo".to_string(),
                        table_name: "repo_entity_repo_beta".to_string(),
                        projected_columns: vec!["name".to_string()],
                    },
                ),
                enqueue_sequence: 0,
            });
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
            task: super::super::types::RepoMaintenanceTask::Compaction(
                super::super::types::RepoCompactionTask {
                    corpus: SearchCorpusKind::RepoEntity,
                    repo_id: "alpha/repo".to_string(),
                    publication_id: "publication-alpha".to_string(),
                    table_name: "repo_entity_repo_alpha".to_string(),
                    row_count: 12,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
            ),
            enqueue_sequence: 1,
        });
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
            task: super::super::types::RepoMaintenanceTask::Compaction(
                super::super::types::RepoCompactionTask {
                    corpus: SearchCorpusKind::RepoContentChunk,
                    repo_id: "gamma/repo".to_string(),
                    publication_id: "publication-gamma".to_string(),
                    table_name: "repo_content_chunk_repo_gamma".to_string(),
                    row_count: 12,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
            ),
            enqueue_sequence: 0,
        });
    }

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(!status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 1);
    assert_eq!(status.maintenance.compaction_queue_position, Some(2));
    assert!(!status.maintenance.compaction_queue_aged);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::CompactionPending);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(reason.readable);
}

#[test]
fn annotate_runtime_status_surfaces_repo_compaction_queue_aging() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction-aged"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));
    {
        let mut runtime = service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.next_enqueue_sequence = 4;
        runtime
            .queue
            .push_back(super::super::types::QueuedRepoMaintenanceTask {
                task: super::super::types::RepoMaintenanceTask::Compaction(
                    super::super::types::RepoCompactionTask {
                        corpus: SearchCorpusKind::RepoEntity,
                        repo_id: "alpha/repo".to_string(),
                        publication_id: "publication-alpha".to_string(),
                        table_name: "repo_entity_repo_alpha".to_string(),
                        row_count: 12,
                        reason:
                            crate::search_plane::coordinator::SearchCompactionReason::RowDeltaRatio,
                    },
                ),
                enqueue_sequence: 0,
            });
    }

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert_eq!(status.maintenance.compaction_queue_depth, 1);
    assert_eq!(status.maintenance.compaction_queue_position, Some(1));
    assert!(status.maintenance.compaction_queue_aged);
}

#[test]
fn enqueue_local_compaction_task_replaces_queued_stale_task_for_same_corpus() {
    let mut runtime = super::super::types::LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 7,
            row_count: 12,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 9,
            row_count: 15,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 1);
    assert!(
        runtime
            .running_compactions
            .contains(&SearchCorpusKind::LocalSymbol)
    );
    let queued = runtime
        .compaction_queue
        .front()
        .unwrap_or_else(|| panic!("queued task should exist"));
    assert_eq!(queued.task.corpus, SearchCorpusKind::LocalSymbol);
    assert_eq!(queued.task.active_epoch, 9);
    assert_eq!(queued.task.row_count, 15);
    assert_eq!(queued.task.reason, SearchCompactionReason::RowDeltaRatio);
}

#[test]
fn enqueue_local_compaction_task_prioritizes_publish_threshold_before_row_delta_ratio() {
    let mut runtime = super::super::types::LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::KnowledgeSection,
            active_epoch: 4,
            row_count: 8,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 9,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 2);
    assert_eq!(
        runtime.compaction_queue[0].task.corpus,
        SearchCorpusKind::LocalSymbol
    );
    assert_eq!(
        runtime.compaction_queue[1].task.corpus,
        SearchCorpusKind::KnowledgeSection
    );
}

#[test]
fn enqueue_local_compaction_task_prioritizes_smaller_row_count_within_same_reason() {
    let mut runtime = super::super::types::LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::KnowledgeSection,
            active_epoch: 4,
            row_count: 64,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 9,
            row_count: 8,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 2);
    assert_eq!(
        runtime.compaction_queue[0].task.corpus,
        SearchCorpusKind::LocalSymbol
    );
    assert_eq!(
        runtime.compaction_queue[1].task.corpus,
        SearchCorpusKind::KnowledgeSection
    );
}

#[test]
fn enqueue_local_compaction_task_ages_row_delta_ratio_ahead_of_new_publish_thresholds() {
    let mut runtime = super::super::types::LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 1,
            row_count: 16,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::KnowledgeSection,
            active_epoch: 2,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::Attachment,
            active_epoch: 3,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        SearchCompactionTask {
            corpus: SearchCorpusKind::ReferenceOccurrence,
            active_epoch: 4,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 4);
    assert_eq!(
        runtime.compaction_queue[2].task.corpus,
        SearchCorpusKind::LocalSymbol
    );
    assert_eq!(
        runtime.compaction_queue[3].task.corpus,
        SearchCorpusKind::ReferenceOccurrence
    );
}
