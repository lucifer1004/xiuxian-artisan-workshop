use serde::{Deserialize, Serialize};
use specta::Type;

/// Search-plane lifecycle phase surfaced to Studio clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexPhase {
    /// No build has been started for the corpus.
    Idle,
    /// A staging epoch is currently being built.
    Indexing,
    /// A published epoch is ready for reads.
    Ready,
    /// A published epoch is readable but partially stale or inconsistent.
    Degraded,
    /// The latest build attempt failed.
    Failed,
}

/// Machine-readable issue code attached to one search corpus status row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexIssueCode {
    /// A repo reported ready but no published manifest exists for this corpus.
    PublishedManifestMissing,
    /// A published manifest exists, but it does not record the source revision.
    PublishedRevisionMissing,
    /// A published manifest exists, but it points at a different source revision.
    PublishedRevisionMismatch,
    /// Repo indexing failed while the corpus status was synthesized.
    RepoIndexFailed,
}

/// High-level issue family used to summarize corpus status for UI consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexIssueFamily {
    /// Issues around missing or malformed published manifests.
    Manifest,
    /// Issues where the published revision no longer matches the repo revision.
    Revision,
    /// Issues coming from repo indexing/sync failures.
    RepoSync,
    /// Multiple issue families are present at once.
    Mixed,
}

/// Machine-readable issue attached to one search corpus status row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexIssue {
    /// Stable issue code suitable for client-side branching.
    pub code: SearchIndexIssueCode,
    /// Whether the corpus remains readable despite this issue.
    pub readable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Repository identifier associated with the issue, when applicable.
    pub repo_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Current repo revision observed during status synthesis.
    pub current_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Published revision currently attached to the serving table, when known.
    pub published_revision: Option<String>,
    /// Human-readable message preserved for current UI surfaces.
    pub message: String,
}

/// High-level summary derived from the issue list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexIssueSummary {
    /// Dominant family for the current issue set.
    pub family: SearchIndexIssueFamily,
    /// Highest-priority issue code in the current issue set.
    pub primary_code: SearchIndexIssueCode,
    /// Total number of issues attached to the corpus.
    pub issue_count: usize,
    /// Number of issues that still allow reads to continue.
    pub readable_issue_count: usize,
}

/// UI-friendly severity for one corpus status reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexStatusSeverity {
    /// Informational lifecycle state.
    Info,
    /// Non-blocking but inconsistent or degraded state.
    Warning,
    /// Blocking state that prevents reliable reads.
    Error,
}

/// Suggested next action for one corpus status reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexStatusAction {
    /// Wait for the in-flight work to finish.
    Wait,
    /// Retry or restart the failed corpus build.
    RetryBuild,
    /// Trigger repo resync/publication rebuild.
    ResyncRepo,
    /// Inspect upstream repo-index sync failures.
    InspectRepoSync,
}

/// Stable machine-readable reason attached to one corpus status row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexStatusReasonCode {
    /// The corpus is indexing for the first time and has no readable publication yet.
    WarmingUp,
    /// The corpus is refreshing while an older publication remains readable.
    Refreshing,
    /// Background compaction has been scheduled for the readable publication.
    CompactionPending,
    /// The latest build failed.
    BuildFailed,
    /// A repo reported ready but no published manifest exists for this corpus.
    PublishedManifestMissing,
    /// A published manifest exists, but it does not record the source revision.
    PublishedRevisionMissing,
    /// A published manifest exists, but it points at a different source revision.
    PublishedRevisionMismatch,
    /// Repo indexing failed while the corpus status was synthesized.
    RepoIndexFailed,
}

/// Compact reason surface that drives UI severity and action semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexStatusReason {
    /// Stable machine-readable reason code.
    pub code: SearchIndexStatusReasonCode,
    /// UI-facing severity lane for the current reason.
    pub severity: SearchIndexStatusSeverity,
    /// Suggested next action for the current reason.
    pub action: SearchIndexStatusAction,
    /// Whether the corpus remains readable despite the current reason.
    pub readable: bool,
}

/// Response-level summary for the dominant corpus status reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexAggregateStatusReason {
    /// Stable machine-readable reason code selected for the aggregate response.
    pub code: SearchIndexStatusReasonCode,
    /// UI-facing severity lane for the selected reason.
    pub severity: SearchIndexStatusSeverity,
    /// Suggested next action for the selected reason.
    pub action: SearchIndexStatusAction,
    /// Number of corpora currently carrying any status reason.
    pub affected_corpus_count: usize,
    /// Number of affected corpora that remain readable.
    pub readable_corpus_count: usize,
    /// Number of affected corpora whose reason is currently blocking reads.
    pub blocking_corpus_count: usize,
}

/// Background maintenance state for one corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexMaintenanceStatus {
    /// Whether the corpus should be compacted in the background.
    pub compaction_pending: bool,
    /// Number of publishes since the last compact.
    pub publish_count_since_compaction: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// RFC3339 timestamp of the latest compaction.
    pub last_compacted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Reason recorded for the latest compaction.
    pub last_compaction_reason: Option<String>,
}

/// Current search-plane status for one corpus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchCorpusIndexStatus {
    /// Stable corpus identifier.
    pub corpus: String,
    /// Current lifecycle phase.
    pub phase: SearchIndexPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Published epoch available to readers.
    pub active_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Staging epoch currently building.
    pub staging_epoch: Option<u64>,
    /// Schema version for the active or in-flight corpus.
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Active or in-flight build fingerprint.
    pub fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Build progress in the range `0.0..=1.0`.
    pub progress: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Published row count for the active epoch.
    pub row_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Published fragment count for the active epoch.
    pub fragment_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// RFC3339 timestamp for the active build start.
    pub build_started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// RFC3339 timestamp for the latest completed build attempt.
    pub build_finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// RFC3339 timestamp for the latest status mutation.
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Latest build error for the corpus.
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Machine-readable issues attached to the current corpus snapshot.
    pub issues: Vec<SearchIndexIssue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// High-level summary derived from the issue list.
    pub issue_summary: Option<SearchIndexIssueSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Compact status reason that folds phase and issues into one UI-friendly decision.
    pub status_reason: Option<SearchIndexStatusReason>,
    /// Maintenance view for the corpus.
    pub maintenance: SearchIndexMaintenanceStatus,
}

/// Aggregated search-plane status payload returned by Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexStatusResponse {
    /// Total number of corpora in the response.
    pub total: usize,
    /// Number of corpora currently idle.
    pub idle: usize,
    /// Number of corpora currently indexing.
    pub indexing: usize,
    /// Number of corpora with ready published epochs.
    pub ready: usize,
    /// Number of corpora with readable but degraded published epochs.
    pub degraded: usize,
    /// Number of corpora whose latest build failed.
    pub failed: usize,
    /// Number of corpora pending compaction.
    pub compaction_pending: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Response-level dominant status reason derived from per-corpus status reasons.
    pub status_reason: Option<SearchIndexAggregateStatusReason>,
    /// Ordered per-corpus status rows.
    pub corpora: Vec<SearchCorpusIndexStatus>,
}

impl From<crate::search_plane::SearchPlanePhase> for SearchIndexPhase {
    fn from(value: crate::search_plane::SearchPlanePhase) -> Self {
        match value {
            crate::search_plane::SearchPlanePhase::Idle => Self::Idle,
            crate::search_plane::SearchPlanePhase::Indexing => Self::Indexing,
            crate::search_plane::SearchPlanePhase::Ready => Self::Ready,
            crate::search_plane::SearchPlanePhase::Degraded => Self::Degraded,
            crate::search_plane::SearchPlanePhase::Failed => Self::Failed,
        }
    }
}

impl From<crate::search_plane::SearchCorpusIssueCode> for SearchIndexIssueCode {
    fn from(value: crate::search_plane::SearchCorpusIssueCode) -> Self {
        match value {
            crate::search_plane::SearchCorpusIssueCode::PublishedManifestMissing => {
                Self::PublishedManifestMissing
            }
            crate::search_plane::SearchCorpusIssueCode::PublishedRevisionMissing => {
                Self::PublishedRevisionMissing
            }
            crate::search_plane::SearchCorpusIssueCode::PublishedRevisionMismatch => {
                Self::PublishedRevisionMismatch
            }
            crate::search_plane::SearchCorpusIssueCode::RepoIndexFailed => Self::RepoIndexFailed,
        }
    }
}

impl From<crate::search_plane::SearchCorpusIssueFamily> for SearchIndexIssueFamily {
    fn from(value: crate::search_plane::SearchCorpusIssueFamily) -> Self {
        match value {
            crate::search_plane::SearchCorpusIssueFamily::Manifest => Self::Manifest,
            crate::search_plane::SearchCorpusIssueFamily::Revision => Self::Revision,
            crate::search_plane::SearchCorpusIssueFamily::RepoSync => Self::RepoSync,
            crate::search_plane::SearchCorpusIssueFamily::Mixed => Self::Mixed,
        }
    }
}

impl From<&crate::search_plane::SearchCorpusIssue> for SearchIndexIssue {
    fn from(value: &crate::search_plane::SearchCorpusIssue) -> Self {
        Self {
            code: value.code.into(),
            readable: value.readable,
            repo_id: value.repo_id.clone(),
            current_revision: value.current_revision.clone(),
            published_revision: value.published_revision.clone(),
            message: value.message.clone(),
        }
    }
}

impl From<&crate::search_plane::SearchCorpusIssueSummary> for SearchIndexIssueSummary {
    fn from(value: &crate::search_plane::SearchCorpusIssueSummary) -> Self {
        Self {
            family: value.family.into(),
            primary_code: value.primary_code.into(),
            issue_count: value.issue_count,
            readable_issue_count: value.readable_issue_count,
        }
    }
}

impl From<crate::search_plane::SearchCorpusStatusSeverity> for SearchIndexStatusSeverity {
    fn from(value: crate::search_plane::SearchCorpusStatusSeverity) -> Self {
        match value {
            crate::search_plane::SearchCorpusStatusSeverity::Info => Self::Info,
            crate::search_plane::SearchCorpusStatusSeverity::Warning => Self::Warning,
            crate::search_plane::SearchCorpusStatusSeverity::Error => Self::Error,
        }
    }
}

impl From<crate::search_plane::SearchCorpusStatusAction> for SearchIndexStatusAction {
    fn from(value: crate::search_plane::SearchCorpusStatusAction) -> Self {
        match value {
            crate::search_plane::SearchCorpusStatusAction::Wait => Self::Wait,
            crate::search_plane::SearchCorpusStatusAction::RetryBuild => Self::RetryBuild,
            crate::search_plane::SearchCorpusStatusAction::ResyncRepo => Self::ResyncRepo,
            crate::search_plane::SearchCorpusStatusAction::InspectRepoSync => Self::InspectRepoSync,
        }
    }
}

impl From<crate::search_plane::SearchCorpusStatusReasonCode> for SearchIndexStatusReasonCode {
    fn from(value: crate::search_plane::SearchCorpusStatusReasonCode) -> Self {
        match value {
            crate::search_plane::SearchCorpusStatusReasonCode::WarmingUp => Self::WarmingUp,
            crate::search_plane::SearchCorpusStatusReasonCode::Refreshing => Self::Refreshing,
            crate::search_plane::SearchCorpusStatusReasonCode::CompactionPending => {
                Self::CompactionPending
            }
            crate::search_plane::SearchCorpusStatusReasonCode::BuildFailed => Self::BuildFailed,
            crate::search_plane::SearchCorpusStatusReasonCode::PublishedManifestMissing => {
                Self::PublishedManifestMissing
            }
            crate::search_plane::SearchCorpusStatusReasonCode::PublishedRevisionMissing => {
                Self::PublishedRevisionMissing
            }
            crate::search_plane::SearchCorpusStatusReasonCode::PublishedRevisionMismatch => {
                Self::PublishedRevisionMismatch
            }
            crate::search_plane::SearchCorpusStatusReasonCode::RepoIndexFailed => {
                Self::RepoIndexFailed
            }
        }
    }
}

impl From<&crate::search_plane::SearchCorpusStatusReason> for SearchIndexStatusReason {
    fn from(value: &crate::search_plane::SearchCorpusStatusReason) -> Self {
        Self {
            code: value.code.into(),
            severity: value.severity.into(),
            action: value.action.into(),
            readable: value.readable,
        }
    }
}

impl From<&crate::search_plane::SearchMaintenanceStatus> for SearchIndexMaintenanceStatus {
    fn from(value: &crate::search_plane::SearchMaintenanceStatus) -> Self {
        Self {
            compaction_pending: value.compaction_pending,
            publish_count_since_compaction: value.publish_count_since_compaction,
            last_compacted_at: value.last_compacted_at.clone(),
            last_compaction_reason: value.last_compaction_reason.clone(),
        }
    }
}

impl From<&crate::search_plane::SearchCorpusStatus> for SearchCorpusIndexStatus {
    fn from(value: &crate::search_plane::SearchCorpusStatus) -> Self {
        Self {
            corpus: value.corpus.to_string(),
            phase: value.phase.into(),
            active_epoch: value.active_epoch,
            staging_epoch: value.staging_epoch,
            schema_version: value.schema_version,
            fingerprint: value.fingerprint.clone(),
            progress: value.progress,
            row_count: value.row_count,
            fragment_count: value.fragment_count,
            build_started_at: value.build_started_at.clone(),
            build_finished_at: value.build_finished_at.clone(),
            updated_at: value.updated_at.clone(),
            last_error: value.last_error.clone(),
            issues: value.issues.iter().map(SearchIndexIssue::from).collect(),
            issue_summary: value
                .issue_summary
                .as_ref()
                .map(SearchIndexIssueSummary::from),
            status_reason: value
                .status_reason
                .as_ref()
                .map(SearchIndexStatusReason::from),
            maintenance: SearchIndexMaintenanceStatus::from(&value.maintenance),
        }
    }
}

impl From<&crate::search_plane::SearchPlaneStatusSnapshot> for SearchIndexStatusResponse {
    fn from(value: &crate::search_plane::SearchPlaneStatusSnapshot) -> Self {
        let corpora = value
            .corpora
            .iter()
            .map(SearchCorpusIndexStatus::from)
            .collect::<Vec<_>>();
        let total = corpora.len();
        let idle = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Idle))
            .count();
        let indexing = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Indexing))
            .count();
        let ready = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Ready))
            .count();
        let failed = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Failed))
            .count();
        let degraded = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Degraded))
            .count();
        let compaction_pending = corpora
            .iter()
            .filter(|status| status.maintenance.compaction_pending)
            .count();
        let status_reason = summarize_response_status_reason(&corpora);
        Self {
            total,
            idle,
            indexing,
            ready,
            degraded,
            failed,
            compaction_pending,
            status_reason,
            corpora,
        }
    }
}

fn summarize_response_status_reason(
    corpora: &[SearchCorpusIndexStatus],
) -> Option<SearchIndexAggregateStatusReason> {
    let reasons = corpora
        .iter()
        .filter_map(|status| status.status_reason.as_ref())
        .collect::<Vec<_>>();
    let primary = reasons.into_iter().min_by_key(|reason| {
        (
            response_reason_severity_priority(reason.severity),
            response_reason_code_priority(reason.code),
        )
    })?;
    let affected_corpus_count = corpora
        .iter()
        .filter(|status| status.status_reason.is_some())
        .count();
    let readable_corpus_count = corpora
        .iter()
        .filter_map(|status| status.status_reason.as_ref())
        .filter(|reason| reason.readable)
        .count();
    let blocking_corpus_count = affected_corpus_count.saturating_sub(readable_corpus_count);
    Some(SearchIndexAggregateStatusReason {
        code: primary.code,
        severity: primary.severity,
        action: primary.action,
        affected_corpus_count,
        readable_corpus_count,
        blocking_corpus_count,
    })
}

fn response_reason_severity_priority(severity: SearchIndexStatusSeverity) -> u8 {
    match severity {
        SearchIndexStatusSeverity::Error => 0,
        SearchIndexStatusSeverity::Warning => 1,
        SearchIndexStatusSeverity::Info => 2,
    }
}

fn response_reason_code_priority(code: SearchIndexStatusReasonCode) -> u8 {
    match code {
        SearchIndexStatusReasonCode::PublishedManifestMissing => 0,
        SearchIndexStatusReasonCode::BuildFailed => 1,
        SearchIndexStatusReasonCode::PublishedRevisionMissing => 2,
        SearchIndexStatusReasonCode::PublishedRevisionMismatch => 3,
        SearchIndexStatusReasonCode::RepoIndexFailed => 4,
        SearchIndexStatusReasonCode::WarmingUp => 5,
        SearchIndexStatusReasonCode::Refreshing => 6,
        SearchIndexStatusReasonCode::CompactionPending => 7,
    }
}

#[cfg(test)]
mod tests {
    use crate::search_plane::{
        SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusKind,
        SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
        SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchMaintenanceStatus,
        SearchPlanePhase, SearchPlaneStatusSnapshot,
    };

    use super::{
        SearchIndexIssueCode, SearchIndexIssueFamily, SearchIndexPhase, SearchIndexStatusAction,
        SearchIndexStatusReasonCode, SearchIndexStatusResponse, SearchIndexStatusSeverity,
    };

    #[test]
    fn response_counts_track_phase_and_compaction_state() {
        let mut local_symbol = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
        local_symbol.phase = SearchPlanePhase::Ready;
        local_symbol.active_epoch = Some(3);
        local_symbol.row_count = Some(10);
        local_symbol.maintenance = SearchMaintenanceStatus {
            compaction_pending: true,
            publish_count_since_compaction: 3,
            last_compacted_at: None,
            last_compaction_reason: None,
        };
        local_symbol.status_reason = Some(SearchCorpusStatusReason {
            code: SearchCorpusStatusReasonCode::CompactionPending,
            severity: SearchCorpusStatusSeverity::Info,
            action: SearchCorpusStatusAction::Wait,
            readable: true,
        });

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

        let response = SearchIndexStatusResponse::from(&SearchPlaneStatusSnapshot {
            corpora: vec![local_symbol, repo_entity],
        });

        assert_eq!(response.total, 2);
        assert_eq!(response.idle, 0);
        assert_eq!(response.indexing, 0);
        assert_eq!(response.ready, 1);
        assert_eq!(response.degraded, 1);
        assert_eq!(response.failed, 0);
        assert_eq!(response.compaction_pending, 1);
        let aggregate_reason = response
            .status_reason
            .as_ref()
            .expect("aggregate status reason should be present");
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
        assert_eq!(response.corpora[0].phase, SearchIndexPhase::Ready);
        let local_reason = response.corpora[0]
            .status_reason
            .as_ref()
            .expect("local status reason should be present");
        assert_eq!(
            local_reason.code,
            SearchIndexStatusReasonCode::CompactionPending
        );
        assert_eq!(local_reason.severity, SearchIndexStatusSeverity::Info);
        assert_eq!(local_reason.action, SearchIndexStatusAction::Wait);
        assert!(local_reason.readable);
        assert_eq!(response.corpora[1].issues.len(), 1);
        assert_eq!(
            response.corpora[1].issues[0].code,
            SearchIndexIssueCode::PublishedRevisionMismatch
        );
        let summary = response.corpora[1]
            .issue_summary
            .as_ref()
            .expect("issue summary should be present");
        assert_eq!(summary.family, SearchIndexIssueFamily::Revision);
        assert_eq!(
            summary.primary_code,
            SearchIndexIssueCode::PublishedRevisionMismatch
        );
        assert_eq!(summary.issue_count, 1);
        assert_eq!(summary.readable_issue_count, 1);
        let reason = response.corpora[1]
            .status_reason
            .as_ref()
            .expect("status reason should be present");
        assert_eq!(
            reason.code,
            SearchIndexStatusReasonCode::PublishedRevisionMismatch
        );
        assert_eq!(reason.severity, SearchIndexStatusSeverity::Warning);
        assert_eq!(reason.action, SearchIndexStatusAction::ResyncRepo);
        assert!(reason.readable);
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
            compaction_pending: true,
            publish_count_since_compaction: 2,
            last_compacted_at: None,
            last_compaction_reason: None,
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

        let aggregate_reason = response
            .status_reason
            .as_ref()
            .expect("aggregate status reason should be present");
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
}
