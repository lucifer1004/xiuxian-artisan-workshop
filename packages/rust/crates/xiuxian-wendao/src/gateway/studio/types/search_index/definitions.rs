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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexIssueFamily {
    /// Issues around missing or malformed published state.
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

/// Response-level summary derived from per-corpus maintenance state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexAggregateMaintenanceSummary {
    /// Number of corpora currently running a prewarm.
    pub prewarm_running_count: usize,
    /// Number of corpora with queued prewarm backlog.
    pub prewarm_queued_corpus_count: usize,
    /// Largest queued prewarm depth observed across corpora.
    pub max_prewarm_queue_depth: u32,
    /// Number of corpora currently running compaction.
    pub compaction_running_count: usize,
    /// Number of corpora with queued compaction backlog.
    pub compaction_queued_corpus_count: usize,
    /// Largest queued compaction depth observed across corpora.
    pub max_compaction_queue_depth: u32,
    /// Number of corpora whose maintenance still reports compaction pending.
    pub compaction_pending_count: usize,
    /// Number of corpora whose queued compaction has already crossed the fairness aging guard.
    pub aged_compaction_queue_count: usize,
}

/// Background maintenance state for one corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexMaintenanceStatus {
    #[serde(default)]
    /// Whether a staging-table prewarm task is actively running.
    pub prewarm_running: bool,
    #[serde(default)]
    /// Number of queued prewarm tasks currently waiting behind the active worker.
    pub prewarm_queue_depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// One-based queue position for this corpus when its prewarm is queued in repo maintenance.
    pub prewarm_queue_position: Option<u32>,
    /// Whether the corpus is actively being compacted in the background.
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
    /// Whether the corpus should be compacted in the background.
    pub compaction_pending: bool,
    /// Number of publishes since the last compact.
    pub publish_count_since_compaction: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// RFC3339 timestamp of the latest staging-table prewarm.
    pub last_prewarmed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Epoch identifier of the latest staging-table prewarm.
    pub last_prewarmed_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// RFC3339 timestamp of the latest compaction.
    pub last_compacted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Reason recorded for the latest compaction.
    pub last_compaction_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Row count observed when the latest compaction completed.
    pub last_compacted_row_count: Option<u64>,
}

/// Source path used by the most recent bounded streaming query for one corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexQueryTelemetrySource {
    /// The query streamed batches from a regular projected scan only.
    Scan,
    /// The query streamed batches from FTS only.
    Fts,
    /// The query attempted FTS first and then fell back to a regular projected scan.
    FtsFallbackScan,
}

/// Recent bounded-rerank telemetry recorded for one corpus query lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexQueryTelemetry {
    /// RFC3339 timestamp when the telemetry record was captured.
    pub captured_at: String,
    /// Optional scope hint such as a repo identifier for repo-backed queries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Streaming source used by the query.
    pub source: SearchIndexQueryTelemetrySource,
    /// Number of streamed batches consumed by the query.
    pub batch_count: u64,
    /// Total number of rows scanned across all streamed batches.
    pub rows_scanned: u64,
    /// Number of rows that matched the lexical predicate before bounded trimming.
    pub matched_rows: u64,
    /// Final number of retained results returned to the caller.
    pub result_count: u64,
    /// Batch row limit used for projected scan requests, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_row_limit: Option<u64>,
    /// Recall limit pushed into the Lance scan/FTS layer, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// Response-level summary derived from the most recent per-corpus query telemetry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexAggregateQueryTelemetry {
    /// Number of corpora contributing recent query telemetry.
    pub corpus_count: usize,
    /// RFC3339 timestamp of the most recent telemetry record in the response.
    pub latest_captured_at: String,
    /// Number of corpora whose most recent query used a projected scan only.
    pub scan_count: usize,
    /// Number of corpora whose most recent query used FTS only.
    pub fts_count: usize,
    /// Number of corpora whose most recent query fell back from FTS to projected scan.
    pub fts_fallback_scan_count: usize,
    /// Total rows scanned across the retained telemetry set.
    pub total_rows_scanned: u64,
    /// Total lexical matches observed before bounded trimming.
    pub total_matched_rows: u64,
    /// Total retained results returned by the recorded queries.
    pub total_result_count: u64,
    /// Maximum batch row limit observed across the retained telemetry set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_batch_row_limit: Option<u64>,
    /// Maximum recall limit observed across the retained telemetry set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_recall_limit_rows: Option<u64>,
    /// Largest working-set budget observed across the retained telemetry set.
    pub max_working_set_budget_rows: u64,
    /// Largest trim threshold observed across the retained telemetry set.
    pub max_trim_threshold_rows: u64,
    /// Largest observed peak working set across the retained telemetry set.
    pub max_peak_working_set_rows: u64,
    /// Total number of trim events observed across the retained telemetry set.
    pub total_trim_count: u64,
    /// Total number of dropped candidates/paths observed across the retained telemetry set.
    pub total_dropped_candidate_count: u64,
    /// Per-scope rollups for telemetry rows carrying a non-empty scope hint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<SearchIndexQueryTelemetryScopeSummary>,
}

/// Response-level telemetry rollup for one concrete scope hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexQueryTelemetryScopeSummary {
    /// Opaque scope hint observed on the contributing telemetry rows.
    pub scope: String,
    /// Number of corpora contributing recent query telemetry for this scope.
    pub corpus_count: usize,
    /// RFC3339 timestamp of the most recent telemetry record in this scope bucket.
    pub latest_captured_at: String,
    /// Number of corpora in this scope bucket whose most recent query used a projected scan only.
    pub scan_count: usize,
    /// Number of corpora in this scope bucket whose most recent query used FTS only.
    pub fts_count: usize,
    /// Number of corpora in this scope bucket whose most recent query fell back from FTS to projected scan.
    pub fts_fallback_scan_count: usize,
    /// Total rows scanned across the retained telemetry set for this scope bucket.
    pub total_rows_scanned: u64,
    /// Total lexical matches observed before bounded trimming for this scope bucket.
    pub total_matched_rows: u64,
    /// Total retained results returned by the recorded queries for this scope bucket.
    pub total_result_count: u64,
    /// Maximum batch row limit observed across this scope bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_batch_row_limit: Option<u64>,
    /// Maximum recall limit observed across this scope bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_recall_limit_rows: Option<u64>,
    /// Largest working-set budget observed across this scope bucket.
    pub max_working_set_budget_rows: u64,
    /// Largest trim threshold observed across this scope bucket.
    pub max_trim_threshold_rows: u64,
    /// Largest observed peak working set across this scope bucket.
    pub max_peak_working_set_rows: u64,
    /// Total number of trim events observed across this scope bucket.
    pub total_trim_count: u64,
    /// Total number of dropped candidates/paths observed across this scope bucket.
    pub total_dropped_candidate_count: u64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Recent bounded-rerank telemetry captured from the last successful query on this corpus.
    pub last_query_telemetry: Option<SearchIndexQueryTelemetry>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Response-level maintenance rollup derived from per-corpus maintenance state.
    pub maintenance_summary: Option<SearchIndexAggregateMaintenanceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Response-level rollup derived from recent per-corpus bounded query telemetry.
    pub query_telemetry_summary: Option<SearchIndexAggregateQueryTelemetry>,
    /// Ordered per-corpus status rows.
    pub corpora: Vec<SearchCorpusIndexStatus>,
}
