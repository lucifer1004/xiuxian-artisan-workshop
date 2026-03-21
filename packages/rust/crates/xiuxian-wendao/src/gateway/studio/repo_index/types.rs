use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::analyzers::RepositoryAnalysisOutput;

/// Lifecycle phase for one background repo-index task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RepoIndexPhase {
    /// No indexing work is currently scheduled for the repository.
    Idle,
    /// The repository is queued for background processing.
    Queued,
    /// The repository source is being validated.
    Checking,
    /// The repository source is being synchronized.
    Syncing,
    /// Analysis and code-document collection are in progress.
    Indexing,
    /// A usable snapshot is ready for search.
    Ready,
    /// The repository configuration or layout is unsupported.
    Unsupported,
    /// The most recent indexing attempt failed.
    Failed,
}

/// Current index status for one configured repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoIndexEntryStatus {
    /// Stable repository identifier.
    pub repo_id: String,
    /// Current lifecycle phase for the repository.
    pub phase: RepoIndexPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Most recent indexing error when the phase is `Failed`.
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Most recent synchronized revision observed for the repository.
    pub last_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Timestamp of the most recent status update.
    pub updated_at: Option<String>,
    /// Number of indexing attempts recorded for the repository.
    pub attempt_count: usize,
}

/// Aggregated repo-index status payload returned to Studio clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoIndexStatusResponse {
    /// Total number of repositories in the response.
    pub total: usize,
    /// Number of repositories currently queued.
    pub queued: usize,
    /// Number of repositories currently being validated.
    pub checking: usize,
    /// Number of repositories currently being synchronized.
    pub syncing: usize,
    /// Number of repositories currently being indexed.
    pub indexing: usize,
    /// Number of repositories with ready snapshots.
    pub ready: usize,
    /// Number of repositories classified as unsupported.
    pub unsupported: usize,
    /// Number of repositories whose latest indexing attempt failed.
    pub failed: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Repository currently being processed, when known.
    pub current_repo_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Per-repository status rows included in the response.
    pub repos: Vec<RepoIndexEntryStatus>,
}

/// Request payload for repo-index enqueue operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoIndexRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional repository identifier to target a single repo.
    pub repo: Option<String>,
    #[serde(default)]
    /// Whether to force a refresh even when a repo is already indexed.
    pub refresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoCodeDocument {
    pub path: String,
    pub language: Option<String>,
    pub contents: Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct RepoIndexSnapshot {
    pub repo_id: String,
    pub analysis: Arc<RepositoryAnalysisOutput>,
    pub code_documents: Arc<Vec<RepoCodeDocument>>,
}
