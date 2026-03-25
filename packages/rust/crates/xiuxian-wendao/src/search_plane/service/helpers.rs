use std::path::{Path, PathBuf};

use xiuxian_io::PrjDirs;

use super::SearchPlaneService;
use super::core::RepoRuntimeState;
use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search_plane::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusIssueSummary,
    SearchCorpusKind, SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
    SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchManifestKeyspace,
    SearchPlanePhase, SearchRepoPublicationRecord,
};

pub(super) fn default_storage_root(project_root: &Path) -> PathBuf {
    PrjDirs::data_home()
        .join("wendao")
        .join("search_plane")
        .join(project_hash(project_root))
}

pub(super) fn manifest_keyspace_for_project(project_root: &Path) -> SearchManifestKeyspace {
    SearchManifestKeyspace::new(format!(
        "xiuxian:wendao:search_plane:{}",
        project_hash(project_root)
    ))
}

pub(super) fn project_hash(project_root: &Path) -> String {
    blake3::hash(project_root.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

pub(super) fn repo_content_phase(
    has_ready_tables: bool,
    has_active_work: bool,
    has_failures: bool,
) -> SearchPlanePhase {
    if has_active_work {
        return SearchPlanePhase::Indexing;
    }
    if has_ready_tables && has_failures {
        return SearchPlanePhase::Degraded;
    }
    if has_ready_tables {
        return SearchPlanePhase::Ready;
    }
    if has_failures {
        return SearchPlanePhase::Failed;
    }
    SearchPlanePhase::Idle
}

pub(super) fn update_latest_timestamp(target: &mut Option<String>, candidate: Option<&str>) {
    let Some(candidate) = candidate else {
        return;
    };
    if target.as_deref().is_none_or(|current| current < candidate) {
        *target = Some(candidate.to_string());
    }
}

pub(super) fn annotate_status_reason(status: &mut SearchCorpusStatus) {
    status.status_reason = derive_status_reason(status);
}

pub(super) fn join_issue_messages(issues: &[SearchCorpusIssue]) -> Option<String> {
    if issues.is_empty() {
        return None;
    }
    Some(
        issues
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

pub(super) fn derive_status_reason(
    status: &SearchCorpusStatus,
) -> Option<SearchCorpusStatusReason> {
    if let Some(summary) = status.issue_summary.as_ref() {
        let readable = status_is_readable(status);
        return Some(SearchCorpusStatusReason {
            code: reason_code_for_issue(summary.primary_code),
            severity: reason_severity_for_issue(summary.primary_code, readable),
            action: reason_action_for_issue(summary.primary_code),
            readable,
        });
    }

    match status.phase {
        SearchPlanePhase::Indexing => Some(SearchCorpusStatusReason {
            code: if status_is_readable(status) {
                SearchCorpusStatusReasonCode::Refreshing
            } else if status.maintenance.prewarm_running || status_has_prewarmed_staging(status) {
                SearchCorpusStatusReasonCode::Prewarming
            } else {
                SearchCorpusStatusReasonCode::WarmingUp
            },
            severity: SearchCorpusStatusSeverity::Info,
            action: SearchCorpusStatusAction::Wait,
            readable: status_is_readable(status),
        }),
        SearchPlanePhase::Failed => {
            let readable = status_is_readable(status);
            Some(SearchCorpusStatusReason {
                code: SearchCorpusStatusReasonCode::BuildFailed,
                severity: if readable {
                    SearchCorpusStatusSeverity::Warning
                } else {
                    SearchCorpusStatusSeverity::Error
                },
                action: SearchCorpusStatusAction::RetryBuild,
                readable,
            })
        }
        SearchPlanePhase::Ready => {
            if status.maintenance.compaction_running {
                Some(SearchCorpusStatusReason {
                    code: SearchCorpusStatusReasonCode::Compacting,
                    severity: SearchCorpusStatusSeverity::Info,
                    action: SearchCorpusStatusAction::Wait,
                    readable: true,
                })
            } else {
                status
                    .maintenance
                    .compaction_pending
                    .then_some(SearchCorpusStatusReason {
                        code: SearchCorpusStatusReasonCode::CompactionPending,
                        severity: SearchCorpusStatusSeverity::Info,
                        action: SearchCorpusStatusAction::Wait,
                        readable: true,
                    })
            }
        }
        SearchPlanePhase::Idle | SearchPlanePhase::Degraded => None,
    }
}

fn status_has_prewarmed_staging(status: &SearchCorpusStatus) -> bool {
    status
        .staging_epoch
        .zip(status.maintenance.last_prewarmed_epoch)
        .is_some_and(|(staging_epoch, prewarmed_epoch)| staging_epoch == prewarmed_epoch)
}

pub(super) fn summarize_issues(issues: &[SearchCorpusIssue]) -> Option<SearchCorpusIssueSummary> {
    let first = issues.first()?;
    let mut family = issue_family(first.code);
    let mut primary_code = first.code;
    let mut readable_issue_count = usize::from(first.readable);
    for issue in issues.iter().skip(1) {
        let current_family = issue_family(issue.code);
        if family != current_family {
            family = SearchCorpusIssueFamily::Mixed;
        }
        if issue_priority(issue.code) < issue_priority(primary_code) {
            primary_code = issue.code;
        }
        if issue.readable {
            readable_issue_count = readable_issue_count.saturating_add(1);
        }
    }
    Some(SearchCorpusIssueSummary {
        family,
        primary_code,
        issue_count: issues.len(),
        readable_issue_count,
    })
}

pub(super) fn status_is_readable(status: &SearchCorpusStatus) -> bool {
    status.active_epoch.is_some()
        || status.row_count.is_some()
        || matches!(
            status.phase,
            SearchPlanePhase::Ready | SearchPlanePhase::Degraded
        )
}

pub(super) fn reason_code_for_issue(code: SearchCorpusIssueCode) -> SearchCorpusStatusReasonCode {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing => {
            SearchCorpusStatusReasonCode::PublishedManifestMissing
        }
        SearchCorpusIssueCode::PublishedRevisionMissing => {
            SearchCorpusStatusReasonCode::PublishedRevisionMissing
        }
        SearchCorpusIssueCode::PublishedRevisionMismatch => {
            SearchCorpusStatusReasonCode::PublishedRevisionMismatch
        }
        SearchCorpusIssueCode::RepoIndexFailed => SearchCorpusStatusReasonCode::RepoIndexFailed,
    }
}

pub(super) fn reason_action_for_issue(code: SearchCorpusIssueCode) -> SearchCorpusStatusAction {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing
        | SearchCorpusIssueCode::PublishedRevisionMissing
        | SearchCorpusIssueCode::PublishedRevisionMismatch => SearchCorpusStatusAction::ResyncRepo,
        SearchCorpusIssueCode::RepoIndexFailed => SearchCorpusStatusAction::InspectRepoSync,
    }
}

pub(super) fn reason_severity_for_issue(
    code: SearchCorpusIssueCode,
    readable: bool,
) -> SearchCorpusStatusSeverity {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing
        | SearchCorpusIssueCode::PublishedRevisionMissing
        | SearchCorpusIssueCode::PublishedRevisionMismatch
        | SearchCorpusIssueCode::RepoIndexFailed => {
            if readable {
                SearchCorpusStatusSeverity::Warning
            } else {
                SearchCorpusStatusSeverity::Error
            }
        }
    }
}

pub(super) fn issue_family(code: SearchCorpusIssueCode) -> SearchCorpusIssueFamily {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing
        | SearchCorpusIssueCode::PublishedRevisionMissing => SearchCorpusIssueFamily::Manifest,
        SearchCorpusIssueCode::PublishedRevisionMismatch => SearchCorpusIssueFamily::Revision,
        SearchCorpusIssueCode::RepoIndexFailed => SearchCorpusIssueFamily::RepoSync,
    }
}

pub(super) fn issue_priority(code: SearchCorpusIssueCode) -> u8 {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing => 0,
        SearchCorpusIssueCode::PublishedRevisionMissing => 1,
        SearchCorpusIssueCode::PublishedRevisionMismatch => 2,
        SearchCorpusIssueCode::RepoIndexFailed => 3,
    }
}

pub(super) fn repo_corpus_fingerprint_part(
    repo: &RepoIndexEntryStatus,
    publication: &SearchRepoPublicationRecord,
) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        repo.repo_id,
        publication.source_revision.as_deref().unwrap_or_default(),
        repo_phase_cache_fragment(repo.phase),
        repo.last_revision.as_deref().unwrap_or_default(),
        publication.table_version_id,
        publication.row_count,
        publication.fragment_count
    )
}

pub(super) fn repo_corpus_active_epoch(
    corpus: SearchCorpusKind,
    publication_epochs: &[u64],
) -> u64 {
    let mut sorted_epochs = publication_epochs.to_vec();
    sorted_epochs.sort_unstable();
    sorted_epochs.dedup();
    stable_epoch_token(
        format!(
            "{corpus}:active:{}",
            sorted_epochs
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join("|")
        )
        .as_str(),
    )
}

pub(super) fn repo_corpus_staging_epoch(
    corpus: SearchCorpusKind,
    repo_statuses: &[RepoIndexEntryStatus],
    active_epoch: Option<u64>,
) -> Option<u64> {
    let mut active_parts = repo_statuses
        .iter()
        .filter(|repo| {
            matches!(
                repo.phase,
                RepoIndexPhase::Queued
                    | RepoIndexPhase::Checking
                    | RepoIndexPhase::Syncing
                    | RepoIndexPhase::Indexing
            )
        })
        .map(|repo| {
            format!(
                "{}:{}:{}:{}",
                repo.repo_id,
                repo_phase_cache_fragment(repo.phase),
                repo.last_revision.as_deref().unwrap_or_default(),
                repo.updated_at.as_deref().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    if active_parts.is_empty() {
        return None;
    }
    active_parts.sort_unstable();
    Some(stable_epoch_token(
        format!(
            "{corpus}:staging:{}:{}",
            active_epoch.unwrap_or_default(),
            active_parts.join("|")
        )
        .as_str(),
    ))
}

pub(super) fn stable_epoch_token(payload: &str) -> u64 {
    let hash = blake3::hash(payload.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_be_bytes(bytes)
}

impl SearchPlaneService {
    pub(super) fn corpus_cache_version(&self, corpus: SearchCorpusKind) -> String {
        let status = self.coordinator().status_for(corpus);
        if let Some(epoch) = status.active_epoch {
            return format!("{corpus}:schema:{}:epoch:{epoch}", corpus.schema_version());
        }
        format!(
            "{corpus}:schema:{}:phase:{}",
            corpus.schema_version(),
            search_phase_cache_fragment(status.phase)
        )
    }
}

pub(super) fn repo_corpus_cache_version(
    corpus: SearchCorpusKind,
    repo_id: &str,
    status: Option<&RepoRuntimeState>,
) -> String {
    let Some(status) = status else {
        return format!(
            "{corpus}:schema:{}:repo:{}:phase:missing",
            corpus.schema_version(),
            normalize_cache_fragment(repo_id)
        );
    };
    format!(
        "{corpus}:schema:{}:repo:{}:phase:{}:revision:{}:updated:{}",
        corpus.schema_version(),
        normalize_cache_fragment(repo_id),
        repo_phase_cache_fragment(status.phase),
        normalize_cache_fragment(status.last_revision.as_deref().unwrap_or_default()),
        normalize_cache_fragment(status.updated_at.as_deref().unwrap_or_default())
    )
}

pub(super) fn repo_publication_cache_version(
    status: Option<&RepoRuntimeState>,
    publication: &SearchRepoPublicationRecord,
) -> String {
    let base = publication.cache_version();
    let Some(status) = status else {
        return base;
    };
    let published_revision =
        normalize_cache_fragment(publication.source_revision.as_deref().unwrap_or_default());
    let current_revision =
        normalize_cache_fragment(status.last_revision.as_deref().unwrap_or_default());
    if status.phase == RepoIndexPhase::Ready
        && (current_revision.is_empty() || current_revision == published_revision)
    {
        return base;
    }
    format!(
        "{base}:phase:{}:current-revision:{current_revision}:published-revision:{published_revision}",
        repo_phase_cache_fragment(status.phase)
    )
}

pub(super) fn repo_manifest_missing_issue(
    corpus: SearchCorpusKind,
    repo: &RepoIndexEntryStatus,
) -> SearchCorpusIssue {
    SearchCorpusIssue {
        code: SearchCorpusIssueCode::PublishedManifestMissing,
        readable: false,
        repo_id: Some(repo.repo_id.clone()),
        current_revision: repo.last_revision.clone(),
        published_revision: None,
        message: format!(
            "{}: published state for {} is missing",
            repo.repo_id,
            corpus.as_str()
        ),
    }
}

pub(super) fn repo_index_failure_issue(
    repo: &RepoIndexEntryStatus,
    publication: Option<&SearchRepoPublicationRecord>,
) -> Option<SearchCorpusIssue> {
    let message = repo.last_error.as_ref()?.clone();
    Some(SearchCorpusIssue {
        code: SearchCorpusIssueCode::RepoIndexFailed,
        readable: publication.is_some(),
        repo_id: Some(repo.repo_id.clone()),
        current_revision: repo.last_revision.clone(),
        published_revision: publication.and_then(|publication| publication.source_revision.clone()),
        message: format!("{}: {message}", repo.repo_id),
    })
}

pub(super) fn repo_publication_consistency_issue(
    corpus: SearchCorpusKind,
    repo: &RepoIndexEntryStatus,
    publication: &SearchRepoPublicationRecord,
) -> Option<SearchCorpusIssue> {
    if repo.phase != RepoIndexPhase::Ready {
        return None;
    }
    let current_revision = repo
        .last_revision
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    let published_revision = publication
        .source_revision
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if current_revision.is_empty() && published_revision.is_empty() {
        return None;
    }
    if published_revision.is_empty() {
        return Some(SearchCorpusIssue {
            code: SearchCorpusIssueCode::PublishedRevisionMissing,
            readable: true,
            repo_id: Some(repo.repo_id.clone()),
            current_revision: repo.last_revision.clone(),
            published_revision: publication.source_revision.clone(),
            message: format!(
                "{}: published state for {} is missing source revision while repo is ready at `{}`",
                repo.repo_id,
                corpus.as_str(),
                current_revision
            ),
        });
    }
    if current_revision.is_empty() || current_revision == published_revision {
        return None;
    }
    Some(SearchCorpusIssue {
        code: SearchCorpusIssueCode::PublishedRevisionMismatch,
        readable: true,
        repo_id: Some(repo.repo_id.clone()),
        current_revision: repo.last_revision.clone(),
        published_revision: publication.source_revision.clone(),
        message: format!(
            "{}: published state for {} targets revision `{published_revision}` but repo is ready at `{current_revision}`",
            repo.repo_id,
            corpus.as_str()
        ),
    })
}

pub(super) fn repo_phase_cache_fragment(phase: RepoIndexPhase) -> &'static str {
    match phase {
        RepoIndexPhase::Idle => "idle",
        RepoIndexPhase::Queued => "queued",
        RepoIndexPhase::Checking => "checking",
        RepoIndexPhase::Syncing => "syncing",
        RepoIndexPhase::Indexing => "indexing",
        RepoIndexPhase::Ready => "ready",
        RepoIndexPhase::Unsupported => "unsupported",
        RepoIndexPhase::Failed => "failed",
    }
}

pub(super) fn search_phase_cache_fragment(phase: SearchPlanePhase) -> &'static str {
    match phase {
        SearchPlanePhase::Idle => "idle",
        SearchPlanePhase::Indexing => "indexing",
        SearchPlanePhase::Ready => "ready",
        SearchPlanePhase::Degraded => "degraded",
        SearchPlanePhase::Failed => "failed",
    }
}

pub(super) fn normalize_cache_fragment(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
