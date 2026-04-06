//! Compatibility wrapper over `xiuxian-git-repo` for Wendao callers.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::analyzers::config::{RegisteredRepository, RepositoryRef, RepositoryRefreshPolicy};
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::query::RepoSyncDriftState;

/// Synchronization mode for repository checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepositorySyncMode {
    /// Ensure checkout exists and is up to date.
    #[default]
    Ensure,
    /// Force refresh from remote.
    Refresh,
    /// Report status without making changes.
    Status,
}

/// Backward-compatible alias used by the analysis service.
pub type CheckoutSyncMode = RepositorySyncMode;

/// Metadata discovered from a local checkout.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCheckoutMetadata {
    /// Current revision of the checkout.
    pub revision: Option<String>,
    /// Upstream remote URL when the checkout is a git repository.
    pub remote_url: Option<String>,
}

/// Lifecycle state of a managed repository.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepositoryLifecycleState {
    /// No lifecycle phase was required.
    NotApplicable,
    /// The expected asset is missing.
    Missing,
    /// A local checkout was validated without mutation.
    Validated,
    /// An existing asset was observed without mutation.
    Observed,
    /// A new asset was created.
    Created,
    /// An existing asset was reused.
    Reused,
    /// An existing asset was refreshed.
    Refreshed,
}

/// Resolved source information for a repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedRepositorySource {
    /// Root path of the checkout.
    pub checkout_root: PathBuf,
    /// Optional managed mirror path.
    pub mirror_root: Option<PathBuf>,
    /// Revision of the mirror (if managed).
    pub mirror_revision: Option<String>,
    /// Revision being tracked.
    pub tracking_revision: Option<String>,
    /// Last observed fetch timestamp in RFC3339 format.
    pub last_fetched_at: Option<String>,
    /// Drift summary between tracked and checkout revisions.
    pub drift_state: RepoSyncDriftState,
    /// Mirror lifecycle state.
    pub mirror_state: RepositoryLifecycleState,
    /// Checkout lifecycle state.
    pub checkout_state: RepositoryLifecycleState,
    /// Kind of the resolved source.
    pub source_kind: ResolvedRepositorySourceKind,
}

/// Kind of resolved repository source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolvedRepositorySourceKind {
    /// Local checkout path provided by user.
    LocalCheckout,
    /// Managed remote repository materialized under `PRJ_DATA_HOME`.
    ManagedRemote,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManagedRemoteProbeStatus {
    #[default]
    Success,
    RetryableFailure,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ManagedRemoteProbeState {
    pub(crate) checked_at: String,
    #[serde(default)]
    pub(crate) status: ManagedRemoteProbeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_success_checked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_success_target_revision: Option<String>,
}

/// Discovers metadata from a local checkout path.
#[must_use]
pub fn discover_checkout_metadata(path: &Path) -> Option<LocalCheckoutMetadata> {
    xiuxian_git_repo::discover_checkout_metadata(path).map(Into::into)
}

pub(crate) fn discover_managed_remote_probe_state(
    mirror_root: &Path,
) -> Option<ManagedRemoteProbeState> {
    xiuxian_git_repo::discover_managed_remote_probe_state(mirror_root).map(Into::into)
}

#[cfg(test)]
pub(crate) fn record_managed_remote_probe_state(
    mirror_root: &Path,
    target_revision: Option<&str>,
) -> std::io::Result<()> {
    xiuxian_git_repo::record_managed_remote_probe_state(mirror_root, target_revision)
}

#[cfg(test)]
pub(crate) fn record_managed_remote_probe_failure(
    mirror_root: &Path,
    error_message: &str,
    retryable: bool,
) -> std::io::Result<()> {
    xiuxian_git_repo::record_managed_remote_probe_failure(mirror_root, error_message, retryable)
}

/// Resolves the source for a registered repository.
///
/// # Errors
///
/// Returns an error when the repository has no usable local path or remote
/// source, or when the selected source cannot be resolved.
pub fn resolve_repository_source(
    repository: &RegisteredRepository,
    cwd: &Path,
    mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    let spec = repo_spec_from_registered(repository);
    xiuxian_git_repo::resolve_repository_source(&spec, cwd, mode.into())
        .map(Into::into)
        .map_err(|error| map_repo_error(repository, error))
}

fn repo_spec_from_registered(repository: &RegisteredRepository) -> xiuxian_git_repo::RepoSpec {
    xiuxian_git_repo::RepoSpec {
        id: repository.id.clone(),
        local_path: repository.path.clone(),
        remote_url: repository.url.clone(),
        revision: repository.git_ref.as_ref().map(repo_revision_selector),
        refresh: repo_refresh_policy(repository.refresh),
    }
}

fn repo_revision_selector(revision: &RepositoryRef) -> xiuxian_git_repo::RevisionSelector {
    match revision {
        RepositoryRef::Branch(value) => xiuxian_git_repo::RevisionSelector::Branch(value.clone()),
        RepositoryRef::Tag(value) => xiuxian_git_repo::RevisionSelector::Tag(value.clone()),
        RepositoryRef::Commit(value) => xiuxian_git_repo::RevisionSelector::Commit(value.clone()),
    }
}

fn repo_refresh_policy(policy: RepositoryRefreshPolicy) -> xiuxian_git_repo::RepoRefreshPolicy {
    match policy {
        RepositoryRefreshPolicy::Fetch => xiuxian_git_repo::RepoRefreshPolicy::Fetch,
        RepositoryRefreshPolicy::Manual => xiuxian_git_repo::RepoRefreshPolicy::Manual,
    }
}

fn map_repo_error(
    repository: &RegisteredRepository,
    error: xiuxian_git_repo::RepoError,
) -> RepoIntelligenceError {
    match error.kind {
        xiuxian_git_repo::RepoErrorKind::MissingSource => {
            RepoIntelligenceError::MissingRepositorySource {
                repo_id: repository.id.clone(),
            }
        }
        xiuxian_git_repo::RepoErrorKind::InvalidPath => {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: repository
                    .path
                    .as_ref()
                    .map_or_else(String::new, |path| path.display().to_string()),
                reason: error.message,
            }
        }
        xiuxian_git_repo::RepoErrorKind::Unsupported => {
            RepoIntelligenceError::UnsupportedRepositoryLayout {
                repo_id: repository.id.clone(),
                message: error.message,
            }
        }
        _ => RepoIntelligenceError::AnalysisFailed {
            message: error.message,
        },
    }
}

impl From<RepositorySyncMode> for xiuxian_git_repo::SyncMode {
    fn from(value: RepositorySyncMode) -> Self {
        match value {
            RepositorySyncMode::Ensure => Self::Ensure,
            RepositorySyncMode::Refresh => Self::Refresh,
            RepositorySyncMode::Status => Self::Status,
        }
    }
}

impl From<xiuxian_git_repo::LocalCheckoutMetadata> for LocalCheckoutMetadata {
    fn from(value: xiuxian_git_repo::LocalCheckoutMetadata) -> Self {
        Self {
            revision: value.revision,
            remote_url: value.remote_url,
        }
    }
}

impl From<xiuxian_git_repo::RepoLifecycleState> for RepositoryLifecycleState {
    fn from(value: xiuxian_git_repo::RepoLifecycleState) -> Self {
        match value {
            xiuxian_git_repo::RepoLifecycleState::NotApplicable => Self::NotApplicable,
            xiuxian_git_repo::RepoLifecycleState::Missing => Self::Missing,
            xiuxian_git_repo::RepoLifecycleState::Validated => Self::Validated,
            xiuxian_git_repo::RepoLifecycleState::Observed => Self::Observed,
            xiuxian_git_repo::RepoLifecycleState::Created => Self::Created,
            xiuxian_git_repo::RepoLifecycleState::Reused => Self::Reused,
            xiuxian_git_repo::RepoLifecycleState::Refreshed => Self::Refreshed,
        }
    }
}

impl From<xiuxian_git_repo::RepoSourceKind> for ResolvedRepositorySourceKind {
    fn from(value: xiuxian_git_repo::RepoSourceKind) -> Self {
        match value {
            xiuxian_git_repo::RepoSourceKind::LocalCheckout => Self::LocalCheckout,
            xiuxian_git_repo::RepoSourceKind::ManagedRemote => Self::ManagedRemote,
        }
    }
}

impl From<xiuxian_git_repo::RepoDriftState> for RepoSyncDriftState {
    fn from(value: xiuxian_git_repo::RepoDriftState) -> Self {
        match value {
            xiuxian_git_repo::RepoDriftState::NotApplicable => Self::NotApplicable,
            xiuxian_git_repo::RepoDriftState::Unknown => Self::Unknown,
            xiuxian_git_repo::RepoDriftState::InSync => Self::InSync,
            xiuxian_git_repo::RepoDriftState::Ahead => Self::Ahead,
            xiuxian_git_repo::RepoDriftState::Behind => Self::Behind,
            xiuxian_git_repo::RepoDriftState::Diverged => Self::Diverged,
        }
    }
}

impl From<xiuxian_git_repo::MaterializedRepo> for ResolvedRepositorySource {
    fn from(value: xiuxian_git_repo::MaterializedRepo) -> Self {
        Self {
            checkout_root: value.checkout_root,
            mirror_root: value.mirror_root,
            mirror_revision: value.mirror_revision,
            tracking_revision: value.tracking_revision,
            last_fetched_at: value.last_fetched_at,
            drift_state: value.drift_state.into(),
            mirror_state: value.mirror_state.into(),
            checkout_state: value.checkout_state.into(),
            source_kind: value.source_kind.into(),
        }
    }
}

impl From<xiuxian_git_repo::ManagedRemoteProbeStatus> for ManagedRemoteProbeStatus {
    fn from(value: xiuxian_git_repo::ManagedRemoteProbeStatus) -> Self {
        match value {
            xiuxian_git_repo::ManagedRemoteProbeStatus::Success => Self::Success,
            xiuxian_git_repo::ManagedRemoteProbeStatus::RetryableFailure => Self::RetryableFailure,
            xiuxian_git_repo::ManagedRemoteProbeStatus::Failure => Self::Failure,
        }
    }
}

impl From<xiuxian_git_repo::ManagedRemoteProbeState> for ManagedRemoteProbeState {
    fn from(value: xiuxian_git_repo::ManagedRemoteProbeState) -> Self {
        Self {
            checked_at: value.checked_at,
            status: value.status.into(),
            target_revision: value.target_revision,
            error_message: value.error_message,
            last_success_checked_at: value.last_success_checked_at,
            last_success_target_revision: value.last_success_target_revision,
        }
    }
}
