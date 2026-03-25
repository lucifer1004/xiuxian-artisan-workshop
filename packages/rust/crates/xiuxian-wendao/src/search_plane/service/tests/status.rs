use crate::search_plane::service::tests::support::*;

fn sample_repo_documents() -> Vec<RepoCodeDocument> {
    vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        size_bytes: 34,
        modified_unix_ms: 0,
    }]
}

fn ready_repo_status(repo_id: &str) -> RepoIndexStatusResponse {
    RepoIndexStatusResponse {
        total: 1,
        active: 0,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 1,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![repo_status_entry(repo_id, RepoIndexPhase::Ready)],
    }
}

fn assert_revision_mismatch_status(
    status: &SearchCorpusStatus,
    repo_id: &str,
    current_revision: &str,
    published_revision: &str,
) {
    assert_eq!(status.phase, SearchPlanePhase::Degraded);
    assert!(status.last_error.as_deref().is_some_and(|message| {
        message.contains(&format!("targets revision `{published_revision}`"))
    }));
    assert_eq!(status.issues.len(), 1);
    assert_eq!(
        status.issues[0].code,
        SearchCorpusIssueCode::PublishedRevisionMismatch
    );
    assert_eq!(status.issues[0].repo_id.as_deref(), Some(repo_id));
    assert_eq!(
        status.issues[0].current_revision.as_deref(),
        Some(current_revision)
    );
    assert_eq!(
        status.issues[0].published_revision.as_deref(),
        Some(published_revision)
    );
    assert!(status.issues[0].readable);

    let summary = issue_summary(status, "issue summary should be present");
    assert_eq!(summary.family, SearchCorpusIssueFamily::Revision);
    assert_eq!(
        summary.primary_code,
        SearchCorpusIssueCode::PublishedRevisionMismatch
    );
    assert_eq!(summary.issue_count, 1);
    assert_eq!(summary.readable_issue_count, 1);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::PublishedRevisionMismatch,
        SearchCorpusStatusSeverity::Warning,
        SearchCorpusStatusAction::ResyncRepo,
        true,
    );
}

fn assert_manifest_missing_status(
    status: &SearchCorpusStatus,
    repo_id: &str,
    current_revision: &str,
) {
    assert_eq!(status.phase, SearchPlanePhase::Failed);
    assert!(status.row_count.is_none());
    assert!(status.fragment_count.is_none());
    assert!(status.fingerprint.is_none());
    assert!(
        status
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("published state"))
    );
    assert_eq!(status.issues.len(), 1);
    assert_eq!(
        status.issues[0].code,
        SearchCorpusIssueCode::PublishedManifestMissing
    );
    assert_eq!(status.issues[0].repo_id.as_deref(), Some(repo_id));
    assert_eq!(
        status.issues[0].current_revision.as_deref(),
        Some(current_revision)
    );
    assert_eq!(status.issues[0].published_revision, None);
    assert!(!status.issues[0].readable);

    let summary = issue_summary(status, "issue summary should be present");
    assert_eq!(summary.family, SearchCorpusIssueFamily::Manifest);
    assert_eq!(
        summary.primary_code,
        SearchCorpusIssueCode::PublishedManifestMissing
    );
    assert_eq!(summary.issue_count, 1);
    assert_eq!(summary.readable_issue_count, 0);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::PublishedManifestMissing,
        SearchCorpusStatusSeverity::Error,
        SearchCorpusStatusAction::ResyncRepo,
        false,
    );
}

#[tokio::test]
async fn status_with_repo_content_surfaces_ready_repo_tables() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        size_bytes: 34,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-1")).await;

    let status = service
        .status_with_repo_content(&RepoIndexStatusResponse {
            total: 1,
            active: 0,
            queued: 0,
            checking: 0,
            syncing: 0,
            indexing: 0,
            ready: 1,
            unsupported: 0,
            failed: 0,
            target_concurrency: 1,
            max_concurrency: 1,
            sync_concurrency_limit: 1,
            current_repo_id: None,
            active_repo_ids: Vec::new(),
            repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
        })
        .await;

    let repo_content = corpus_status(
        &status,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_eq!(repo_content.phase, SearchPlanePhase::Ready);
    assert!(repo_content.active_epoch.is_some());
    assert!(repo_content.staging_epoch.is_none());
    assert!(repo_content.row_count.unwrap_or_default() > 0);
    assert!(repo_content.fragment_count.unwrap_or_default() > 0);
    assert!(repo_content.fingerprint.is_some());
    assert!(repo_content.build_finished_at.is_some());
    assert!(repo_content.updated_at.is_some());
    assert!(repo_content.last_error.is_none());
    assert!(repo_content.issues.is_empty());
    assert!(repo_content.issue_summary.is_none());
    assert!(repo_content.status_reason.is_none());

    let repo_entity = corpus_status(
        &status,
        SearchCorpusKind::RepoEntity,
        "repo entity row should exist",
    );
    assert_eq!(repo_entity.phase, SearchPlanePhase::Ready);
    assert!(repo_entity.active_epoch.is_some());
    assert!(repo_entity.staging_epoch.is_none());
    assert!(repo_entity.row_count.unwrap_or_default() > 0);
    assert!(repo_entity.fragment_count.unwrap_or_default() > 0);
    assert!(repo_entity.fingerprint.is_some());
    assert!(repo_entity.build_finished_at.is_some());
    assert!(repo_entity.updated_at.is_some());
    assert!(repo_entity.last_error.is_none());
    assert!(repo_entity.issues.is_empty());
    assert!(repo_entity.issue_summary.is_none());
    assert!(repo_entity.status_reason.is_none());
}

#[tokio::test]
async fn status_snapshot_reuses_last_synchronized_repo_corpus_state() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        size_bytes: 34,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-1")).await;

    service
        .status_with_repo_content(&RepoIndexStatusResponse {
            total: 1,
            active: 0,
            queued: 0,
            checking: 0,
            syncing: 0,
            indexing: 0,
            ready: 1,
            unsupported: 0,
            failed: 0,
            target_concurrency: 1,
            max_concurrency: 1,
            sync_concurrency_limit: 1,
            current_repo_id: None,
            active_repo_ids: Vec::new(),
            repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
        })
        .await;

    let snapshot = service.status();
    let repo_content = corpus_status(
        &snapshot,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_eq!(repo_content.phase, SearchPlanePhase::Ready);
    assert!(repo_content.active_epoch.is_some());
    assert!(repo_content.row_count.unwrap_or_default() > 0);

    let repo_entity = corpus_status(
        &snapshot,
        SearchCorpusKind::RepoEntity,
        "repo entity row should exist",
    );
    assert_eq!(repo_entity.phase, SearchPlanePhase::Ready);
    assert!(repo_entity.active_epoch.is_some());
    assert!(repo_entity.row_count.unwrap_or_default() > 0);
}

#[tokio::test]
async fn status_snapshot_surfaces_last_query_telemetry() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    service.record_query_telemetry(
        SearchCorpusKind::KnowledgeSection,
        SearchQueryTelemetry {
            captured_at: "2026-03-23T22:20:00Z".to_string(),
            scope: None,
            source: SearchQueryTelemetrySource::Fts,
            batch_count: 3,
            rows_scanned: 120,
            matched_rows: 22,
            result_count: 10,
            batch_row_limit: Some(64),
            recall_limit_rows: Some(96),
            working_set_budget_rows: 40,
            trim_threshold_rows: 80,
            peak_working_set_rows: 55,
            trim_count: 1,
            dropped_candidate_count: 6,
        },
    );

    let snapshot = service.status();
    let knowledge = corpus_status(
        &snapshot,
        SearchCorpusKind::KnowledgeSection,
        "knowledge row should exist",
    );
    let telemetry = last_query_telemetry(knowledge, "telemetry should be present");
    assert_eq!(telemetry.captured_at, "2026-03-23T22:20:00Z");
    assert_eq!(telemetry.source, SearchQueryTelemetrySource::Fts);
    assert_eq!(telemetry.batch_count, 3);
    assert_eq!(telemetry.rows_scanned, 120);
    assert_eq!(telemetry.matched_rows, 22);
    assert_eq!(telemetry.result_count, 10);
    assert_eq!(telemetry.batch_row_limit, Some(64));
    assert_eq!(telemetry.recall_limit_rows, Some(96));
    assert_eq!(telemetry.working_set_budget_rows, 40);
    assert_eq!(telemetry.trim_threshold_rows, 80);
    assert_eq!(telemetry.peak_working_set_rows, 55);
    assert_eq!(telemetry.trim_count, 1);
    assert_eq!(telemetry.dropped_candidate_count, 6);
}

#[tokio::test]
async fn status_with_repo_content_reports_indexing_before_publish() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    let status = service
        .status_with_repo_content(&RepoIndexStatusResponse {
            total: 1,
            active: 1,
            queued: 0,
            checking: 0,
            syncing: 0,
            indexing: 1,
            ready: 0,
            unsupported: 0,
            failed: 0,
            target_concurrency: 1,
            max_concurrency: 1,
            sync_concurrency_limit: 1,
            current_repo_id: Some("alpha/repo".to_string()),
            active_repo_ids: vec!["alpha/repo".to_string()],
            repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)],
        })
        .await;

    let repo_content = corpus_status(
        &status,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_eq!(repo_content.phase, SearchPlanePhase::Indexing);
    assert!(repo_content.active_epoch.is_none());
    assert!(repo_content.staging_epoch.is_some());
    assert!(repo_content.row_count.is_none());
    assert!(repo_content.fragment_count.is_none());
    assert!(repo_content.fingerprint.is_none());
    assert!(repo_content.build_finished_at.is_none());
    assert!(repo_content.updated_at.is_some());
    assert!(repo_content.last_error.is_none());
    assert!(repo_content.issues.is_empty());
    assert!(repo_content.issue_summary.is_none());
    assert_status_reason(
        repo_content,
        SearchCorpusStatusReasonCode::WarmingUp,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        false,
    );

    let repo_entity = corpus_status(
        &status,
        SearchCorpusKind::RepoEntity,
        "repo entity row should exist",
    );
    assert_eq!(repo_entity.phase, SearchPlanePhase::Indexing);
    assert!(repo_entity.active_epoch.is_none());
    assert!(repo_entity.staging_epoch.is_some());
    assert!(repo_entity.row_count.is_none());
    assert!(repo_entity.fragment_count.is_none());
    assert!(repo_entity.fingerprint.is_none());
    assert!(repo_entity.build_finished_at.is_none());
    assert!(repo_entity.updated_at.is_some());
    assert!(repo_entity.last_error.is_none());
    assert!(repo_entity.issues.is_empty());
    assert!(repo_entity.issue_summary.is_none());
    assert_status_reason(
        repo_entity,
        SearchCorpusStatusReasonCode::WarmingUp,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        false,
    );
}

#[tokio::test]
async fn status_with_repo_content_keeps_published_metadata_while_repo_refreshes() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        size_bytes: 34,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-0")).await;

    let status = service
        .status_with_repo_content(&RepoIndexStatusResponse {
            total: 1,
            active: 1,
            queued: 0,
            checking: 0,
            syncing: 0,
            indexing: 1,
            ready: 0,
            unsupported: 0,
            failed: 0,
            target_concurrency: 1,
            max_concurrency: 1,
            sync_concurrency_limit: 1,
            current_repo_id: Some("alpha/repo".to_string()),
            active_repo_ids: vec!["alpha/repo".to_string()],
            repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)],
        })
        .await;

    let repo_content = corpus_status(
        &status,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_eq!(repo_content.phase, SearchPlanePhase::Indexing);
    assert!(repo_content.active_epoch.is_some());
    assert!(repo_content.staging_epoch.is_some());
    assert!(repo_content.row_count.unwrap_or_default() > 0);
    assert!(repo_content.fragment_count.unwrap_or_default() > 0);
    assert!(repo_content.fingerprint.is_some());
    assert!(repo_content.build_finished_at.is_some());
    assert!(repo_content.issues.is_empty());
    assert!(repo_content.issue_summary.is_none());
    assert_status_reason(
        repo_content,
        SearchCorpusStatusReasonCode::Refreshing,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );

    let repo_entity = corpus_status(
        &status,
        SearchCorpusKind::RepoEntity,
        "repo entity row should exist",
    );
    assert_eq!(repo_entity.phase, SearchPlanePhase::Indexing);
    assert!(repo_entity.active_epoch.is_some());
    assert!(repo_entity.staging_epoch.is_some());
    assert!(repo_entity.row_count.unwrap_or_default() > 0);
    assert!(repo_entity.fragment_count.unwrap_or_default() > 0);
    assert!(repo_entity.fingerprint.is_some());
    assert!(repo_entity.build_finished_at.is_some());
    assert!(repo_entity.last_error.is_none());
    assert!(repo_entity.issues.is_empty());
    assert!(repo_entity.issue_summary.is_none());
    assert_status_reason(
        repo_entity,
        SearchCorpusStatusReasonCode::Refreshing,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );
}

#[tokio::test]
async fn status_with_repo_content_reports_revision_mismatch_for_ready_repo() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = sample_repo_documents();
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-0")).await;

    let status = service
        .status_with_repo_content(&ready_repo_status("alpha/repo"))
        .await;

    let repo_content = corpus_status(
        &status,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_revision_mismatch_status(repo_content, "alpha/repo", "rev-1", "rev-0");

    let repo_entity = corpus_status(
        &status,
        SearchCorpusKind::RepoEntity,
        "repo entity row should exist",
    );
    assert_revision_mismatch_status(repo_entity, "alpha/repo", "rev-1", "rev-0");
}

#[tokio::test]
async fn status_with_repo_content_requires_published_state_even_when_disk_tables_exist() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = sample_repo_documents();
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-1")).await;
    service.clear_repo_publications("alpha/repo");

    assert!(
        !service
            .has_published_repo_corpus(SearchCorpusKind::RepoEntity, "alpha/repo")
            .await
    );
    assert!(
        !service
            .has_published_repo_corpus(SearchCorpusKind::RepoContentChunk, "alpha/repo")
            .await
    );

    let status = service
        .status_with_repo_content(&ready_repo_status("alpha/repo"))
        .await;

    let repo_content = corpus_status(
        &status,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_manifest_missing_status(repo_content, "alpha/repo", "rev-1");

    let repo_entity = corpus_status(
        &status,
        SearchCorpusKind::RepoEntity,
        "repo entity row should exist",
    );
    assert_manifest_missing_status(repo_entity, "alpha/repo", "rev-1");
}

#[tokio::test]
async fn status_with_repo_content_reports_repo_failure_issue_while_rows_remain_readable() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        size_bytes: 34,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-1")).await;

    let status = service
        .status_with_repo_content(&RepoIndexStatusResponse {
            total: 1,
            active: 0,
            queued: 0,
            checking: 0,
            syncing: 0,
            indexing: 0,
            ready: 0,
            unsupported: 0,
            failed: 1,
            target_concurrency: 1,
            max_concurrency: 1,
            sync_concurrency_limit: 1,
            current_repo_id: None,
            active_repo_ids: Vec::new(),
            repos: vec![RepoIndexEntryStatus {
                last_error: Some("git fetch failed".to_string()),
                ..repo_status_entry("alpha/repo", RepoIndexPhase::Failed)
            }],
        })
        .await;

    let repo_content = corpus_status(
        &status,
        SearchCorpusKind::RepoContentChunk,
        "repo content row should exist",
    );
    assert_eq!(repo_content.phase, SearchPlanePhase::Degraded);
    assert_eq!(repo_content.issues.len(), 1);
    assert_eq!(
        repo_content.issues[0].code,
        SearchCorpusIssueCode::RepoIndexFailed
    );
    assert!(repo_content.issues[0].readable);
    assert_eq!(
        repo_content.issues[0].published_revision.as_deref(),
        Some("rev-1")
    );
    let repo_content_summary = issue_summary(repo_content, "issue summary should be present");
    assert_eq!(
        repo_content_summary.family,
        SearchCorpusIssueFamily::RepoSync
    );
    assert_eq!(
        repo_content_summary.primary_code,
        SearchCorpusIssueCode::RepoIndexFailed
    );
    assert_eq!(repo_content_summary.issue_count, 1);
    assert_eq!(repo_content_summary.readable_issue_count, 1);
    assert_status_reason(
        repo_content,
        SearchCorpusStatusReasonCode::RepoIndexFailed,
        SearchCorpusStatusSeverity::Warning,
        SearchCorpusStatusAction::InspectRepoSync,
        true,
    );
    assert!(
        repo_content
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("git fetch failed"))
    );
}

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

#[test]
fn summarize_issues_prefers_highest_priority_code_and_marks_mixed_family() {
    let summary = some_or_panic(
        summarize_issues(&[
            SearchCorpusIssue {
                code: SearchCorpusIssueCode::RepoIndexFailed,
                readable: true,
                repo_id: Some("alpha/repo".to_string()),
                current_revision: Some("rev-2".to_string()),
                published_revision: Some("rev-1".to_string()),
                message: "alpha/repo: git fetch failed".to_string(),
            },
            SearchCorpusIssue {
                code: SearchCorpusIssueCode::PublishedManifestMissing,
                readable: false,
                repo_id: Some("beta/repo".to_string()),
                current_revision: Some("rev-9".to_string()),
                published_revision: None,
                message: "beta/repo: published state missing".to_string(),
            },
        ]),
        "summary should exist",
    );

    assert_eq!(summary.family, SearchCorpusIssueFamily::Mixed);
    assert_eq!(
        summary.primary_code,
        SearchCorpusIssueCode::PublishedManifestMissing
    );
    assert_eq!(summary.issue_count, 2);
    assert_eq!(summary.readable_issue_count, 1);
}
