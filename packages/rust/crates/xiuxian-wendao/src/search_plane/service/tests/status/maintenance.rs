use crate::search_plane::service::tests::support::*;

#[test]
fn derive_status_reason_marks_failed_refresh_as_retryable_warning() {
    let mut status = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    status.phase = SearchPlanePhase::Failed;
    status.active_epoch = Some(7);
    status.row_count = Some(12);
    status.last_error = Some("builder crashed".to_string());

    let reason = some_or_panic(derive_status_reason(&status), "status reason should exist");

    assert_eq!(reason.code, SearchCorpusStatusReasonCode::BuildFailed);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Warning);
    assert_eq!(reason.action, SearchCorpusStatusAction::RetryBuild);
    assert!(reason.readable);
}

#[test]
fn status_marks_indexing_corpus_with_running_prewarm_reason() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-prewarm-running",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(
        service
            .coordinator()
            .mark_prewarm_running(SearchCorpusKind::LocalSymbol, lease.epoch)
    );

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert_eq!(status.staging_epoch, Some(lease.epoch));
    assert!(status.maintenance.prewarm_running);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::Prewarming,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        false,
    );
}

#[test]
fn status_marks_indexing_corpus_with_prewarmed_staging_reason() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-prewarmed-staging",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(
        service
            .coordinator()
            .mark_prewarm_complete(SearchCorpusKind::LocalSymbol, lease.epoch)
    );

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert_eq!(status.staging_epoch, Some(lease.epoch));
    assert!(!status.maintenance.prewarm_running);
    assert_eq!(status.maintenance.last_prewarmed_epoch, Some(lease.epoch));
    assert!(status.maintenance.last_prewarmed_at.is_some());
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::Prewarming,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        false,
    );
}

#[test]
fn status_marks_ready_corpus_with_pending_compaction_reason() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-compaction-pending",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert!(!status.maintenance.compaction_running);
    assert!(status.maintenance.compaction_pending);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::CompactionPending,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );
}

#[test]
fn status_marks_ready_corpus_with_running_compaction_reason() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-compacting",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));
    service
        .local_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_compaction = Some(SearchCorpusKind::LocalSymbol);

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert!(status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 0);
    assert_eq!(status.maintenance.compaction_queue_position, None);
    assert!(status.maintenance.compaction_pending);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::Compacting,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );
}

#[test]
fn status_surfaces_local_compaction_queue_backlog_for_queued_corpus() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-local-queue",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));
    {
        let mut runtime = service
            .local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.compaction_queue.push_back(
            crate::search_plane::service::core::QueuedLocalCompactionTask {
                task: crate::search_plane::coordinator::SearchCompactionTask {
                    corpus: SearchCorpusKind::KnowledgeSection,
                    active_epoch: 1,
                    row_count: 8,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
                enqueue_sequence: 0,
            },
        );
        runtime.compaction_queue.push_back(
            crate::search_plane::service::core::QueuedLocalCompactionTask {
                task: crate::search_plane::coordinator::SearchCompactionTask {
                    corpus: SearchCorpusKind::LocalSymbol,
                    active_epoch: 1,
                    row_count: 10,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
                enqueue_sequence: 1,
            },
        );
        runtime
            .running_compactions
            .insert(SearchCorpusKind::KnowledgeSection);
        runtime
            .running_compactions
            .insert(SearchCorpusKind::LocalSymbol);
        runtime.worker_running = true;
    }

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert!(!status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 2);
    assert_eq!(status.maintenance.compaction_queue_position, Some(2));
    assert!(!status.maintenance.compaction_queue_aged);
    assert!(status.maintenance.compaction_pending);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::CompactionPending,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );
}

#[test]
fn status_surfaces_local_compaction_queue_aging_for_aged_row_delta_task() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-local-aged-queue",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));
    {
        let mut runtime = service
            .local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.next_enqueue_sequence = 4;
        runtime.compaction_queue.push_back(
            crate::search_plane::service::core::QueuedLocalCompactionTask {
                task: crate::search_plane::coordinator::SearchCompactionTask {
                    corpus: SearchCorpusKind::LocalSymbol,
                    active_epoch: 1,
                    row_count: 10,
                    reason: crate::search_plane::coordinator::SearchCompactionReason::RowDeltaRatio,
                },
                enqueue_sequence: 0,
            },
        );
        runtime
            .running_compactions
            .insert(SearchCorpusKind::LocalSymbol);
        runtime.worker_running = true;
    }

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert_eq!(status.maintenance.compaction_queue_depth, 1);
    assert_eq!(status.maintenance.compaction_queue_position, Some(1));
    assert!(status.maintenance.compaction_queue_aged);
}
