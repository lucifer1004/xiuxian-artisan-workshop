use serde::{Deserialize, Serialize};

use super::SearchCorpusKind;

/// Runtime phase for a search-plane corpus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchPlanePhase {
    /// No build has been started for the corpus.
    #[default]
    Idle,
    /// A staging epoch is being built in the background.
    Indexing,
    /// A published epoch is available for reads.
    Ready,
    /// A published epoch is still readable, but the corpus is partially stale or inconsistent.
    Degraded,
    /// The latest attempted build failed.
    Failed,
}

/// Machine-readable issue code attached to a corpus status row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusIssueCode {
    /// A repo reported ready but no published state exists for this corpus.
    PublishedManifestMissing,
    /// Published state exists, but it does not record the source revision.
    PublishedRevisionMissing,
    /// Published state exists, but it points at a different source revision.
    PublishedRevisionMismatch,
    /// Repo indexing failed while the corpus status was synthesized.
    RepoIndexFailed,
}

/// High-level issue family used to summarize corpus status for UI consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusIssueFamily {
    /// Issues around missing or malformed published state.
    Manifest,
    /// Issues where the published revision no longer matches the repo revision.
    Revision,
    /// Issues coming from repo indexing/sync failures.
    RepoSync,
    /// Multiple issue families are present at once.
    Mixed,
}

/// Machine-readable issue attached to a corpus status row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCorpusIssue {
    /// Stable issue code suitable for client-side branching.
    pub code: SearchCorpusIssueCode,
    /// Whether the corpus remains readable despite this issue.
    pub readable: bool,
    /// Repository identifier associated with the issue, when applicable.
    pub repo_id: Option<String>,
    /// Current repo revision observed during status synthesis.
    pub current_revision: Option<String>,
    /// Published revision currently attached to the serving table, when known.
    pub published_revision: Option<String>,
    /// Human-readable message preserved for logs and current UI surfaces.
    pub message: String,
}

/// High-level summary derived from the corpus issue list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCorpusIssueSummary {
    /// Dominant family for the current issue set.
    pub family: SearchCorpusIssueFamily,
    /// Highest-priority issue code in the current issue set.
    pub primary_code: SearchCorpusIssueCode,
    /// Total number of issues attached to the corpus.
    pub issue_count: usize,
    /// Number of issues that still allow reads to continue.
    pub readable_issue_count: usize,
}

/// UI-friendly severity for one corpus status reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusStatusSeverity {
    /// Informational lifecycle state.
    Info,
    /// Non-blocking but inconsistent or degraded state.
    Warning,
    /// Blocking state that prevents reliable reads.
    Error,
}

/// Suggested next action for one corpus status reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusStatusAction {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusStatusReasonCode {
    /// The corpus is indexing for the first time and has no readable publication yet.
    WarmingUp,
    /// The corpus is indexing for the first time, and the staging epoch has already been prewarmed.
    Prewarming,
    /// The corpus is refreshing while an older publication remains readable.
    Refreshing,
    /// Background compaction is actively running for the readable publication.
    Compacting,
    /// Background compaction has been scheduled for the readable publication.
    CompactionPending,
    /// The latest build failed.
    BuildFailed,
    /// A repo reported ready but no published state exists for this corpus.
    PublishedManifestMissing,
    /// Published state exists, but it does not record the source revision.
    PublishedRevisionMissing,
    /// Published state exists, but it points at a different source revision.
    PublishedRevisionMismatch,
    /// Repo indexing failed while the corpus status was synthesized.
    RepoIndexFailed,
}

/// Compact reason surface that drives UI severity and action semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCorpusStatusReason {
    /// Stable machine-readable reason code.
    pub code: SearchCorpusStatusReasonCode,
    /// UI-facing severity lane for the current reason.
    pub severity: SearchCorpusStatusSeverity,
    /// Suggested next action for the current reason.
    pub action: SearchCorpusStatusAction,
    /// Whether the corpus remains readable despite the current reason.
    pub readable: bool,
}

/// Heuristics for deciding when background compaction should be scheduled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMaintenancePolicy {
    /// Force compaction after this many publishes since the last compact.
    pub publish_count_threshold: u32,
    /// Force compaction when row count drift exceeds this ratio.
    pub row_delta_ratio_threshold: f32,
}

impl SearchMaintenancePolicy {
    /// Return the first compaction reason whose threshold is currently violated.
    #[must_use]
    pub(crate) fn compaction_reason(
        &self,
        publish_count_since_compaction: u32,
        last_compacted_row_count: Option<u64>,
        next_row_count: u64,
    ) -> Option<super::coordinator::SearchCompactionReason> {
        if publish_count_since_compaction >= self.publish_count_threshold {
            return Some(super::coordinator::SearchCompactionReason::PublishThreshold);
        }
        let previous_row_count = last_compacted_row_count?;
        if previous_row_count == 0 {
            return (next_row_count > 0)
                .then_some(super::coordinator::SearchCompactionReason::RowDeltaRatio);
        }
        let delta = previous_row_count.abs_diff(next_row_count);
        let (threshold_numerator, threshold_denominator) =
            ratio_threshold_parts(self.row_delta_ratio_threshold);
        (u128::from(delta) * threshold_denominator
            >= u128::from(previous_row_count) * threshold_numerator)
            .then_some(super::coordinator::SearchCompactionReason::RowDeltaRatio)
    }

    /// Decide whether background compaction should be scheduled.
    #[must_use]
    pub fn should_compact(
        &self,
        publish_count_since_compaction: u32,
        last_compacted_row_count: Option<u64>,
        next_row_count: u64,
    ) -> bool {
        self.compaction_reason(
            publish_count_since_compaction,
            last_compacted_row_count,
            next_row_count,
        )
        .is_some()
    }
}

impl Default for SearchMaintenancePolicy {
    fn default() -> Self {
        Self {
            publish_count_threshold: 8,
            row_delta_ratio_threshold: 0.25,
        }
    }
}

fn ratio_threshold_parts(threshold: f32) -> (u128, u128) {
    let normalized = if threshold.is_sign_negative() {
        String::from("0")
    } else {
        format!("{threshold:.6}")
    };
    let trimmed = normalized.trim_end_matches('0').trim_end_matches('.');
    let (whole_part, fractional_part) = match trimmed.split_once('.') {
        Some((whole_part, fractional_part)) => (whole_part, fractional_part),
        None => (trimmed, ""),
    };
    let whole_value = whole_part.parse::<u128>().ok().unwrap_or_default();
    if fractional_part.is_empty() {
        return (whole_value, 1);
    }
    let denominator = 10_u128.pow(
        u32::try_from(fractional_part.len())
            .ok()
            .unwrap_or_default(),
    );
    let fractional_value = fractional_part.parse::<u128>().ok().unwrap_or_default();
    (
        whole_value
            .saturating_mul(denominator)
            .saturating_add(fractional_value),
        denominator,
    )
}

/// Background maintenance state derived from publish/compaction history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchMaintenanceStatus {
    #[serde(default)]
    /// Whether a staging-table prewarm task is actively running.
    pub prewarm_running: bool,
    #[serde(default)]
    /// Number of queued prewarm tasks currently waiting behind the active worker.
    pub prewarm_queue_depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// One-based queue position for this corpus when its prewarm is queued in repo maintenance.
    pub prewarm_queue_position: Option<u32>,
    /// Whether a compaction task is actively running for the readable publication.
    pub compaction_running: bool,
    #[serde(default)]
    /// Number of queued compaction tasks currently waiting behind the active worker.
    pub compaction_queue_depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// One-based queue position for this corpus when its compaction is queued locally.
    pub compaction_queue_position: Option<u32>,
    #[serde(default)]
    /// Whether enqueue-time fairness aging has already promoted this queued compaction task.
    pub compaction_queue_aged: bool,
    /// Whether the coordinator should schedule a compact/optimize run.
    pub compaction_pending: bool,
    /// Number of publishes observed since the last successful compaction.
    pub publish_count_since_compaction: u32,
    /// RFC3339 timestamp of the most recent successful staging-table prewarm.
    pub last_prewarmed_at: Option<String>,
    /// Epoch identifier of the most recent successful staging-table prewarm.
    pub last_prewarmed_epoch: Option<u64>,
    /// RFC3339 timestamp of the most recent successful compaction.
    pub last_compacted_at: Option<String>,
    /// Human-readable reason for the most recent compaction.
    pub last_compaction_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Row count observed when compaction most recently completed.
    pub last_compacted_row_count: Option<u64>,
}

/// Source path used by the most recent bounded streaming query for one corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchQueryTelemetrySource {
    /// The query streamed batches from a regular projected scan only.
    Scan,
    /// The query streamed batches from FTS only.
    Fts,
    /// The query attempted FTS first and then fell back to a regular projected scan.
    FtsFallbackScan,
}

/// Recent bounded-rerank telemetry recorded for one corpus query lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQueryTelemetry {
    /// RFC3339 timestamp when the telemetry record was captured.
    pub captured_at: String,
    /// Optional scope hint such as a repo identifier for repo-backed queries.
    pub scope: Option<String>,
    /// Streaming source used by the query.
    pub source: SearchQueryTelemetrySource,
    /// Number of streamed batches consumed by the query.
    pub batch_count: u64,
    /// Total number of rows scanned across all streamed batches.
    pub rows_scanned: u64,
    /// Number of rows that matched the lexical predicate before bounded trimming.
    pub matched_rows: u64,
    /// Final number of retained results returned to the caller.
    pub result_count: u64,
    /// Batch row limit used for projected scan requests, when configured.
    pub batch_row_limit: Option<u64>,
    /// Recall limit pushed into the Lance scan/FTS layer, when configured.
    pub recall_limit_rows: Option<u64>,
    /// Soft in-memory working-set budget expressed as retained candidate rows.
    pub working_set_budget_rows: u64,
    /// Trim threshold that triggers bounded compaction of the working set.
    pub trim_threshold_rows: u64,
    /// Largest candidate/path working set observed during the query.
    pub peak_working_set_rows: u64,
    /// Number of times the working set had to be trimmed.
    pub trim_count: u64,
    /// Number of candidates/paths dropped by bounded trimming.
    pub dropped_candidate_count: u64,
}

/// Per-corpus status snapshot for API and orchestration layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchCorpusStatus {
    /// Corpus this status row describes.
    pub corpus: SearchCorpusKind,
    /// Current build/publish phase.
    pub phase: SearchPlanePhase,
    /// Last published epoch available to readers.
    pub active_epoch: Option<u64>,
    /// Current staging epoch being built, if any.
    pub staging_epoch: Option<u64>,
    /// Active schema version expected by the builder and reader.
    pub schema_version: u32,
    /// Fingerprint of the currently active or in-flight build.
    pub fingerprint: Option<String>,
    /// Build progress in the range `0.0..=1.0` while indexing.
    pub progress: Option<f32>,
    /// Published row count for the active epoch.
    pub row_count: Option<u64>,
    /// Published fragment count for the active epoch.
    pub fragment_count: Option<u64>,
    /// RFC3339 timestamp for the current build start.
    pub build_started_at: Option<String>,
    /// RFC3339 timestamp for the latest completed build attempt.
    pub build_finished_at: Option<String>,
    /// RFC3339 timestamp for the latest status mutation.
    pub updated_at: Option<String>,
    /// Last recorded build error, if any.
    pub last_error: Option<String>,
    /// Machine-readable issues attached to the current corpus snapshot.
    pub issues: Vec<SearchCorpusIssue>,
    /// High-level summary derived from the issue list.
    pub issue_summary: Option<SearchCorpusIssueSummary>,
    /// Compact status reason that folds phase and issues into one UI-friendly decision.
    pub status_reason: Option<SearchCorpusStatusReason>,
    /// Recent bounded-rerank telemetry captured from the last successful query on this corpus.
    pub last_query_telemetry: Option<SearchQueryTelemetry>,
    /// Background maintenance state for the corpus.
    pub maintenance: SearchMaintenanceStatus,
}

impl SearchCorpusStatus {
    /// Build an empty status row for a corpus.
    #[must_use]
    pub fn new(corpus: SearchCorpusKind) -> Self {
        Self {
            corpus,
            phase: SearchPlanePhase::Idle,
            active_epoch: None,
            staging_epoch: None,
            schema_version: corpus.schema_version(),
            fingerprint: None,
            progress: None,
            row_count: None,
            fragment_count: None,
            build_started_at: None,
            build_finished_at: None,
            updated_at: None,
            last_error: None,
            issues: Vec::new(),
            issue_summary: None,
            status_reason: None,
            last_query_telemetry: None,
            maintenance: SearchMaintenanceStatus::default(),
        }
    }
}

/// Multi-corpus view returned by the coordinator and status API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchPlaneStatusSnapshot {
    /// Ordered status rows for every search-plane corpus.
    pub corpora: Vec<SearchCorpusStatus>,
}

#[cfg(test)]
mod tests {
    use super::{SearchMaintenancePolicy, ratio_threshold_parts};
    use crate::search_plane::coordinator::SearchCompactionReason;

    #[test]
    fn ratio_threshold_parts_preserves_decimal_thresholds() {
        assert_eq!(ratio_threshold_parts(0.25), (25, 100));
        assert_eq!(ratio_threshold_parts(0.9), (9, 10));
        assert_eq!(ratio_threshold_parts(1.0), (1, 1));
        assert_eq!(ratio_threshold_parts(-1.0), (0, 1));
    }

    #[test]
    fn compaction_reason_uses_fixed_precision_ratio_comparison() {
        let policy = SearchMaintenancePolicy {
            publish_count_threshold: 99,
            row_delta_ratio_threshold: 0.25,
        };

        assert_eq!(policy.compaction_reason(0, Some(100), 124), None);
        assert_eq!(
            policy.compaction_reason(0, Some(100), 125),
            Some(SearchCompactionReason::RowDeltaRatio)
        );
    }
}
