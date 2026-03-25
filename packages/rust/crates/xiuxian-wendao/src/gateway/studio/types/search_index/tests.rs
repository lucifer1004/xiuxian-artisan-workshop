use crate::search_plane::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusKind,
    SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
    SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchMaintenanceStatus,
    SearchPlanePhase, SearchPlaneStatusSnapshot, SearchQueryTelemetry, SearchQueryTelemetrySource,
};

use super::{
    SearchIndexIssueCode, SearchIndexIssueFamily, SearchIndexPhase,
    SearchIndexQueryTelemetrySource, SearchIndexStatusAction, SearchIndexStatusReasonCode,
    SearchIndexStatusResponse, SearchIndexStatusSeverity,
};

fn status_reason(response: &SearchIndexStatusResponse) -> &super::SearchIndexAggregateStatusReason {
    response
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("aggregate status reason should be present"))
}

fn corpus_status_reason(
    response: &SearchIndexStatusResponse,
    index: usize,
) -> &super::SearchIndexStatusReason {
    response.corpora[index]
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should be present"))
}

fn corpus_issue_summary(
    response: &SearchIndexStatusResponse,
    index: usize,
) -> &super::SearchIndexIssueSummary {
    response.corpora[index]
        .issue_summary
        .as_ref()
        .unwrap_or_else(|| panic!("issue summary should be present"))
}

fn compacting_local_symbol_status() -> SearchCorpusStatus {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Ready;
    local_symbol.active_epoch = Some(3);
    local_symbol.row_count = Some(10);
    local_symbol.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: true,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: true,
        publish_count_since_compaction: 3,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };
    local_symbol.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::Compacting,
        severity: SearchCorpusStatusSeverity::Info,
        action: SearchCorpusStatusAction::Wait,
        readable: true,
    });
    local_symbol
}

fn degraded_repo_entity_status() -> SearchCorpusStatus {
    let mut repo_entity = SearchCorpusStatus::new(SearchCorpusKind::RepoEntity);
    repo_entity.phase = SearchPlanePhase::Degraded;
    repo_entity.issues.push(SearchCorpusIssue {
        code: SearchCorpusIssueCode::PublishedRevisionMismatch,
        readable: true,
        repo_id: Some("alpha/repo".to_string()),
        current_revision: Some("rev-2".to_string()),
        published_revision: Some("rev-1".to_string()),
        message: "alpha/repo drifted".to_string(),
    });
    repo_entity.issue_summary = Some(crate::search_plane::SearchCorpusIssueSummary {
        family: SearchCorpusIssueFamily::Revision,
        primary_code: SearchCorpusIssueCode::PublishedRevisionMismatch,
        issue_count: 1,
        readable_issue_count: 1,
    });
    repo_entity.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::PublishedRevisionMismatch,
        severity: SearchCorpusStatusSeverity::Warning,
        action: SearchCorpusStatusAction::ResyncRepo,
        readable: true,
    });
    repo_entity
}

fn telemetry_attachment_status() -> SearchCorpusStatus {
    let mut attachment = SearchCorpusStatus::new(SearchCorpusKind::Attachment);
    attachment.phase = SearchPlanePhase::Ready;
    attachment.active_epoch = Some(9);
    attachment.last_query_telemetry = Some(SearchQueryTelemetry {
        captured_at: "2026-03-23T22:05:00Z".to_string(),
        scope: Some("alpha/repo".to_string()),
        source: SearchQueryTelemetrySource::FtsFallbackScan,
        batch_count: 4,
        rows_scanned: 96,
        matched_rows: 19,
        result_count: 8,
        batch_row_limit: Some(32),
        recall_limit_rows: Some(64),
        working_set_budget_rows: 24,
        trim_threshold_rows: 48,
        peak_working_set_rows: 41,
        trim_count: 2,
        dropped_candidate_count: 11,
    });
    attachment
}

fn telemetry_knowledge_status() -> SearchCorpusStatus {
    let mut knowledge = SearchCorpusStatus::new(SearchCorpusKind::KnowledgeSection);
    knowledge.phase = SearchPlanePhase::Ready;
    knowledge.active_epoch = Some(4);
    knowledge.last_query_telemetry = Some(SearchQueryTelemetry {
        captured_at: "2026-03-23T22:07:00Z".to_string(),
        scope: None,
        source: SearchQueryTelemetrySource::Fts,
        batch_count: 2,
        rows_scanned: 70,
        matched_rows: 14,
        result_count: 6,
        batch_row_limit: Some(16),
        recall_limit_rows: Some(40),
        working_set_budget_rows: 12,
        trim_threshold_rows: 24,
        peak_working_set_rows: 18,
        trim_count: 1,
        dropped_candidate_count: 5,
    });
    knowledge
}

#[test]
fn response_counts_track_phase_and_compaction_state() {
    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![
            compacting_local_symbol_status(),
            degraded_repo_entity_status(),
            telemetry_attachment_status(),
            telemetry_knowledge_status(),
        ],
    });

    assert_eq!(response.total, 4);
    assert_eq!(response.idle, 0);
    assert_eq!(response.indexing, 0);
    assert_eq!(response.ready, 3);
    assert_eq!(response.degraded, 1);
    assert_eq!(response.failed, 0);
    assert_eq!(response.compaction_pending, 1);
    let aggregate_reason = status_reason(&response);
    assert_eq!(
        aggregate_reason.code,
        SearchIndexStatusReasonCode::PublishedRevisionMismatch
    );
    assert_eq!(
        aggregate_reason.severity,
        SearchIndexStatusSeverity::Warning
    );
    assert_eq!(aggregate_reason.action, SearchIndexStatusAction::ResyncRepo);
    assert_eq!(aggregate_reason.affected_corpus_count, 2);
    assert_eq!(aggregate_reason.readable_corpus_count, 2);
    assert_eq!(aggregate_reason.blocking_corpus_count, 0);
    let maintenance_summary = response
        .maintenance_summary
        .as_ref()
        .unwrap_or_else(|| panic!("maintenance summary should be present"));
    assert_eq!(maintenance_summary.prewarm_running_count, 0);
    assert_eq!(maintenance_summary.prewarm_queued_corpus_count, 0);
    assert_eq!(maintenance_summary.max_prewarm_queue_depth, 0);
    assert_eq!(maintenance_summary.compaction_running_count, 1);
    assert_eq!(maintenance_summary.compaction_queued_corpus_count, 0);
    assert_eq!(maintenance_summary.max_compaction_queue_depth, 0);
    assert_eq!(maintenance_summary.compaction_pending_count, 1);
    assert_eq!(maintenance_summary.aged_compaction_queue_count, 0);
    assert_eq!(response.corpora[0].phase, SearchIndexPhase::Ready);
    let local_reason = corpus_status_reason(&response, 0);
    assert_eq!(local_reason.code, SearchIndexStatusReasonCode::Compacting);
    assert_eq!(local_reason.severity, SearchIndexStatusSeverity::Info);
    assert_eq!(local_reason.action, SearchIndexStatusAction::Wait);
    assert!(local_reason.readable);
    assert!(response.corpora[0].maintenance.compaction_running);
    assert_eq!(response.corpora[1].issues.len(), 1);
    assert_eq!(
        response.corpora[1].issues[0].code,
        SearchIndexIssueCode::PublishedRevisionMismatch
    );
    let summary = corpus_issue_summary(&response, 1);
    assert_eq!(summary.family, SearchIndexIssueFamily::Revision);
    assert_eq!(
        summary.primary_code,
        SearchIndexIssueCode::PublishedRevisionMismatch
    );
    assert_eq!(summary.issue_count, 1);
    assert_eq!(summary.readable_issue_count, 1);
    let reason = corpus_status_reason(&response, 1);
    assert_eq!(
        reason.code,
        SearchIndexStatusReasonCode::PublishedRevisionMismatch
    );
    assert_eq!(reason.severity, SearchIndexStatusSeverity::Warning);
    assert_eq!(reason.action, SearchIndexStatusAction::ResyncRepo);
    assert!(reason.readable);
    let telemetry = response.corpora[2]
        .last_query_telemetry
        .as_ref()
        .unwrap_or_else(|| panic!("telemetry should be present"));
    assert_eq!(
        telemetry.source,
        super::SearchIndexQueryTelemetrySource::FtsFallbackScan
    );
    assert_eq!(telemetry.scope.as_deref(), Some("alpha/repo"));
    assert_eq!(telemetry.batch_count, 4);
    assert_eq!(telemetry.rows_scanned, 96);
    assert_eq!(telemetry.matched_rows, 19);
    assert_eq!(telemetry.result_count, 8);
    assert_eq!(telemetry.batch_row_limit, Some(32));
    assert_eq!(telemetry.recall_limit_rows, Some(64));
    assert_eq!(telemetry.working_set_budget_rows, 24);
    assert_eq!(telemetry.trim_threshold_rows, 48);
    assert_eq!(telemetry.peak_working_set_rows, 41);
    assert_eq!(telemetry.trim_count, 2);
    assert_eq!(telemetry.dropped_candidate_count, 11);
    let telemetry_summary = response
        .query_telemetry_summary
        .as_ref()
        .unwrap_or_else(|| panic!("query telemetry summary should be present"));
    assert_eq!(telemetry_summary.corpus_count, 2);
    assert_eq!(telemetry_summary.latest_captured_at, "2026-03-23T22:07:00Z");
    assert_eq!(telemetry_summary.scan_count, 0);
    assert_eq!(telemetry_summary.fts_count, 1);
    assert_eq!(telemetry_summary.fts_fallback_scan_count, 1);
    assert_eq!(telemetry_summary.total_rows_scanned, 166);
    assert_eq!(telemetry_summary.total_matched_rows, 33);
    assert_eq!(telemetry_summary.total_result_count, 14);
    assert_eq!(telemetry_summary.max_batch_row_limit, Some(32));
    assert_eq!(telemetry_summary.max_recall_limit_rows, Some(64));
    assert_eq!(telemetry_summary.max_working_set_budget_rows, 24);
    assert_eq!(telemetry_summary.max_trim_threshold_rows, 48);
    assert_eq!(telemetry_summary.max_peak_working_set_rows, 41);
    assert_eq!(telemetry_summary.total_trim_count, 3);
    assert_eq!(telemetry_summary.total_dropped_candidate_count, 16);
    assert_eq!(telemetry_summary.scopes.len(), 1);
    assert_eq!(telemetry_summary.scopes[0].scope, "alpha/repo");
    assert_eq!(telemetry_summary.scopes[0].corpus_count, 1);
    assert_eq!(
        telemetry_summary.scopes[0].latest_captured_at,
        "2026-03-23T22:05:00Z"
    );
    assert_eq!(telemetry_summary.scopes[0].scan_count, 0);
    assert_eq!(telemetry_summary.scopes[0].fts_count, 0);
    assert_eq!(telemetry_summary.scopes[0].fts_fallback_scan_count, 1);
    assert_eq!(telemetry_summary.scopes[0].total_rows_scanned, 96);
}

#[test]
fn response_status_reason_prefers_blocking_error_over_warning_and_info() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Failed;
    local_symbol.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::BuildFailed,
        severity: SearchCorpusStatusSeverity::Error,
        action: SearchCorpusStatusAction::RetryBuild,
        readable: false,
    });

    let mut knowledge = SearchCorpusStatus::new(SearchCorpusKind::KnowledgeSection);
    knowledge.phase = SearchPlanePhase::Ready;
    knowledge.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: false,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: true,
        publish_count_since_compaction: 2,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };
    knowledge.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::CompactionPending,
        severity: SearchCorpusStatusSeverity::Info,
        action: SearchCorpusStatusAction::Wait,
        readable: true,
    });

    let mut repo_entity = SearchCorpusStatus::new(SearchCorpusKind::RepoEntity);
    repo_entity.phase = SearchPlanePhase::Degraded;
    repo_entity.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::PublishedRevisionMismatch,
        severity: SearchCorpusStatusSeverity::Warning,
        action: SearchCorpusStatusAction::ResyncRepo,
        readable: true,
    });

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol, knowledge, repo_entity],
    });

    let aggregate_reason = status_reason(&response);
    assert_eq!(
        aggregate_reason.code,
        SearchIndexStatusReasonCode::BuildFailed
    );
    assert_eq!(aggregate_reason.severity, SearchIndexStatusSeverity::Error);
    assert_eq!(aggregate_reason.action, SearchIndexStatusAction::RetryBuild);
    assert_eq!(aggregate_reason.affected_corpus_count, 3);
    assert_eq!(aggregate_reason.readable_corpus_count, 2);
    assert_eq!(aggregate_reason.blocking_corpus_count, 1);
}

#[test]
fn response_status_reason_prefers_compacting_over_compaction_pending() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Ready;
    local_symbol.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: true,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: true,
        publish_count_since_compaction: 4,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };
    local_symbol.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::Compacting,
        severity: SearchCorpusStatusSeverity::Info,
        action: SearchCorpusStatusAction::Wait,
        readable: true,
    });

    let mut knowledge = SearchCorpusStatus::new(SearchCorpusKind::KnowledgeSection);
    knowledge.phase = SearchPlanePhase::Ready;
    knowledge.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: false,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };
    knowledge.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::CompactionPending,
        severity: SearchCorpusStatusSeverity::Info,
        action: SearchCorpusStatusAction::Wait,
        readable: true,
    });

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol, knowledge],
    });

    let aggregate_reason = status_reason(&response);
    assert_eq!(
        aggregate_reason.code,
        SearchIndexStatusReasonCode::Compacting
    );
    assert_eq!(aggregate_reason.severity, SearchIndexStatusSeverity::Info);
    assert_eq!(aggregate_reason.action, SearchIndexStatusAction::Wait);
    assert_eq!(aggregate_reason.affected_corpus_count, 2);
    assert_eq!(aggregate_reason.readable_corpus_count, 2);
    assert_eq!(aggregate_reason.blocking_corpus_count, 0);
    assert!(response.maintenance_summary.is_some());
    assert!(response.query_telemetry_summary.is_none());
}

#[test]
fn response_status_reason_prefers_warming_up_over_prewarming() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Indexing;
    local_symbol.staging_epoch = Some(5);
    local_symbol.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::WarmingUp,
        severity: SearchCorpusStatusSeverity::Info,
        action: SearchCorpusStatusAction::Wait,
        readable: false,
    });

    let mut knowledge = SearchCorpusStatus::new(SearchCorpusKind::KnowledgeSection);
    knowledge.phase = SearchPlanePhase::Indexing;
    knowledge.staging_epoch = Some(7);
    knowledge.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: false,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: false,
        publish_count_since_compaction: 0,
        last_prewarmed_at: Some("2026-03-24T12:34:56Z".to_string()),
        last_prewarmed_epoch: Some(7),
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };
    knowledge.status_reason = Some(SearchCorpusStatusReason {
        code: SearchCorpusStatusReasonCode::Prewarming,
        severity: SearchCorpusStatusSeverity::Info,
        action: SearchCorpusStatusAction::Wait,
        readable: false,
    });

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol, knowledge],
    });

    let aggregate_reason = status_reason(&response);
    assert_eq!(
        aggregate_reason.code,
        SearchIndexStatusReasonCode::WarmingUp
    );
    assert_eq!(aggregate_reason.severity, SearchIndexStatusSeverity::Info);
    assert_eq!(aggregate_reason.action, SearchIndexStatusAction::Wait);
    assert_eq!(aggregate_reason.affected_corpus_count, 2);
    assert_eq!(aggregate_reason.readable_corpus_count, 0);
    assert_eq!(aggregate_reason.blocking_corpus_count, 2);
    let prewarming_reason = corpus_status_reason(&response, 1);
    assert_eq!(
        prewarming_reason.code,
        SearchIndexStatusReasonCode::Prewarming
    );
    assert_eq!(prewarming_reason.severity, SearchIndexStatusSeverity::Info);
    assert_eq!(prewarming_reason.action, SearchIndexStatusAction::Wait);
    assert!(!prewarming_reason.readable);
}

#[test]
fn response_maps_prewarm_maintenance_metadata() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Ready;
    local_symbol.maintenance = SearchMaintenanceStatus {
        prewarm_running: true,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: false,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: false,
        publish_count_since_compaction: 1,
        last_prewarmed_at: Some("2026-03-24T12:34:56Z".to_string()),
        last_prewarmed_epoch: Some(7),
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol],
    });

    assert_eq!(
        response.corpora[0].maintenance.last_prewarmed_at.as_deref(),
        Some("2026-03-24T12:34:56Z")
    );
    assert!(response.corpora[0].maintenance.prewarm_running);
    assert_eq!(
        response.corpora[0].maintenance.last_prewarmed_epoch,
        Some(7)
    );
}

#[test]
fn response_maps_local_compaction_queue_metadata() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Ready;
    local_symbol.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: false,
        compaction_queue_depth: 2,
        compaction_queue_position: Some(2),
        compaction_queue_aged: true,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol],
    });

    assert_eq!(response.corpora[0].maintenance.compaction_queue_depth, 2);
    assert_eq!(
        response.corpora[0].maintenance.compaction_queue_position,
        Some(2)
    );
    assert!(response.corpora[0].maintenance.compaction_queue_aged);
}

#[test]
fn response_maps_repo_prewarm_queue_metadata() {
    let mut repo_entity = SearchCorpusStatus::new(SearchCorpusKind::RepoEntity);
    repo_entity.phase = SearchPlanePhase::Indexing;
    repo_entity.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 1,
        prewarm_queue_position: Some(2),
        compaction_running: false,
        compaction_queue_depth: 0,
        compaction_queue_position: None,
        compaction_queue_aged: false,
        compaction_pending: false,
        publish_count_since_compaction: 0,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![repo_entity],
    });

    assert_eq!(response.corpora[0].maintenance.prewarm_queue_depth, 1);
    assert_eq!(
        response.corpora[0].maintenance.prewarm_queue_position,
        Some(2)
    );
}

#[test]
fn response_maintenance_summary_rolls_up_queue_and_aging_state() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Ready;
    local_symbol.maintenance = SearchMaintenanceStatus {
        prewarm_running: false,
        prewarm_queue_depth: 0,
        prewarm_queue_position: None,
        compaction_running: false,
        compaction_queue_depth: 2,
        compaction_queue_position: Some(2),
        compaction_queue_aged: true,
        compaction_pending: true,
        publish_count_since_compaction: 3,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };

    let mut repo_entity = SearchCorpusStatus::new(SearchCorpusKind::RepoEntity);
    repo_entity.phase = SearchPlanePhase::Indexing;
    repo_entity.maintenance = SearchMaintenanceStatus {
        prewarm_running: true,
        prewarm_queue_depth: 1,
        prewarm_queue_position: Some(1),
        compaction_running: true,
        compaction_queue_depth: 1,
        compaction_queue_position: Some(1),
        compaction_queue_aged: false,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        last_prewarmed_at: None,
        last_prewarmed_epoch: None,
        last_compacted_at: None,
        last_compaction_reason: None,
        last_compacted_row_count: None,
    };

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol, repo_entity],
    });

    let summary = response
        .maintenance_summary
        .as_ref()
        .unwrap_or_else(|| panic!("maintenance summary should be present"));
    assert_eq!(summary.prewarm_running_count, 1);
    assert_eq!(summary.prewarm_queued_corpus_count, 1);
    assert_eq!(summary.max_prewarm_queue_depth, 1);
    assert_eq!(summary.compaction_running_count, 1);
    assert_eq!(summary.compaction_queued_corpus_count, 2);
    assert_eq!(summary.max_compaction_queue_depth, 2);
    assert_eq!(summary.compaction_pending_count, 2);
    assert_eq!(summary.aged_compaction_queue_count, 1);
}

#[test]
fn response_maintenance_summary_stays_empty_without_signals() {
    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![
            SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol),
            SearchCorpusStatus::new(SearchCorpusKind::Attachment),
        ],
    });

    assert!(response.maintenance_summary.is_none());
}

#[test]
fn response_query_telemetry_summary_remains_empty_without_corpus_telemetry() {
    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![
            SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol),
            SearchCorpusStatus::new(SearchCorpusKind::Attachment),
        ],
    });

    assert!(response.query_telemetry_summary.is_none());
}

#[test]
fn response_query_telemetry_summary_preserves_source_mapping() {
    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![telemetry_attachment_status()],
    });

    let summary = response
        .query_telemetry_summary
        .as_ref()
        .unwrap_or_else(|| panic!("query telemetry summary should be present"));
    assert_eq!(summary.scan_count, 0);
    assert_eq!(summary.fts_count, 0);
    assert_eq!(summary.fts_fallback_scan_count, 1);
    assert_eq!(summary.scopes.len(), 1);
    assert_eq!(summary.scopes[0].scope, "alpha/repo");
    assert_eq!(summary.scopes[0].fts_fallback_scan_count, 1);
    assert_eq!(
        response.corpora[0]
            .last_query_telemetry
            .as_ref()
            .map(|telemetry| telemetry.source),
        Some(SearchIndexQueryTelemetrySource::FtsFallbackScan)
    );
}

#[test]
fn response_query_telemetry_summary_groups_rows_by_scope_hint() {
    let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
    local_symbol.phase = SearchPlanePhase::Ready;
    local_symbol.last_query_telemetry = Some(SearchQueryTelemetry {
        captured_at: "2026-03-23T22:10:00Z".to_string(),
        scope: Some("autocomplete".to_string()),
        source: SearchQueryTelemetrySource::Scan,
        batch_count: 2,
        rows_scanned: 25,
        matched_rows: 9,
        result_count: 5,
        batch_row_limit: Some(16),
        recall_limit_rows: Some(32),
        working_set_budget_rows: 12,
        trim_threshold_rows: 24,
        peak_working_set_rows: 14,
        trim_count: 1,
        dropped_candidate_count: 3,
    });

    let mut reference = SearchCorpusStatus::new(SearchCorpusKind::ReferenceOccurrence);
    reference.phase = SearchPlanePhase::Ready;
    reference.last_query_telemetry = Some(SearchQueryTelemetry {
        captured_at: "2026-03-23T22:11:00Z".to_string(),
        scope: Some("search".to_string()),
        source: SearchQueryTelemetrySource::Fts,
        batch_count: 3,
        rows_scanned: 40,
        matched_rows: 12,
        result_count: 6,
        batch_row_limit: Some(24),
        recall_limit_rows: Some(48),
        working_set_budget_rows: 18,
        trim_threshold_rows: 36,
        peak_working_set_rows: 21,
        trim_count: 0,
        dropped_candidate_count: 0,
    });

    let mut attachment = SearchCorpusStatus::new(SearchCorpusKind::Attachment);
    attachment.phase = SearchPlanePhase::Ready;
    attachment.last_query_telemetry = Some(SearchQueryTelemetry {
        captured_at: "2026-03-23T22:12:00Z".to_string(),
        scope: Some("search".to_string()),
        source: SearchQueryTelemetrySource::FtsFallbackScan,
        batch_count: 4,
        rows_scanned: 60,
        matched_rows: 15,
        result_count: 7,
        batch_row_limit: Some(32),
        recall_limit_rows: Some(64),
        working_set_budget_rows: 24,
        trim_threshold_rows: 48,
        peak_working_set_rows: 29,
        trim_count: 2,
        dropped_candidate_count: 5,
    });

    let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
        corpora: vec![local_symbol, reference, attachment],
    });

    let summary = response
        .query_telemetry_summary
        .as_ref()
        .unwrap_or_else(|| panic!("query telemetry summary should be present"));
    assert_eq!(summary.corpus_count, 3);
    assert_eq!(summary.scopes.len(), 2);
    assert_eq!(summary.scopes[0].scope, "autocomplete");
    assert_eq!(summary.scopes[0].corpus_count, 1);
    assert_eq!(summary.scopes[0].scan_count, 1);
    assert_eq!(summary.scopes[0].fts_count, 0);
    assert_eq!(summary.scopes[0].fts_fallback_scan_count, 0);
    assert_eq!(summary.scopes[0].total_rows_scanned, 25);
    assert_eq!(summary.scopes[1].scope, "search");
    assert_eq!(summary.scopes[1].corpus_count, 2);
    assert_eq!(summary.scopes[1].scan_count, 0);
    assert_eq!(summary.scopes[1].fts_count, 1);
    assert_eq!(summary.scopes[1].fts_fallback_scan_count, 1);
    assert_eq!(summary.scopes[1].total_rows_scanned, 100);
    assert_eq!(summary.scopes[1].total_matched_rows, 27);
    assert_eq!(summary.scopes[1].total_result_count, 13);
    assert_eq!(summary.scopes[1].max_batch_row_limit, Some(32));
    assert_eq!(summary.scopes[1].max_recall_limit_rows, Some(64));
    assert_eq!(summary.scopes[1].total_trim_count, 2);
    assert_eq!(summary.scopes[1].total_dropped_candidate_count, 5);
}
