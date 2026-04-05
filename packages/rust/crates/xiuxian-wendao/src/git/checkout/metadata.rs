use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use chrono::Utc;
use git2::{Oid, Repository};
use serde::{Deserialize, Serialize};

use crate::analyzers::config::RepositoryRef;
use crate::analyzers::query::RepoSyncDriftState;

use super::LocalCheckoutMetadata;

const MANAGED_REMOTE_PROBE_STATE_FILE: &str = "xiuxian-upstream-probe-state.json";

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
    if !path.is_dir() {
        return None;
    }

    let repository = Repository::open(path).ok()?;
    Some(LocalCheckoutMetadata {
        revision: resolve_head_revision(&repository),
        remote_url: repository
            .find_remote("origin")
            .ok()
            .and_then(|remote| remote.url().map(str::to_string)),
    })
}

pub(super) fn resolve_head_revision(repository: &Repository) -> Option<String> {
    repository
        .head()
        .ok()
        .and_then(|head| head.target().map(|oid| oid.to_string()))
}

pub(super) fn resolve_tracking_revision(
    repository: &Repository,
    git_ref: Option<&RepositoryRef>,
) -> Option<String> {
    match git_ref {
        Some(RepositoryRef::Commit(sha)) => Some(sha.clone()),
        Some(RepositoryRef::Tag(tag)) => repository
            .find_reference(format!("refs/tags/{tag}").as_str())
            .ok()
            .and_then(|reference| reference.target().map(|oid| oid.to_string())),
        Some(RepositoryRef::Branch(branch)) => repository
            .find_reference(format!("refs/remotes/origin/{branch}").as_str())
            .ok()
            .and_then(|reference| reference.target().map(|oid| oid.to_string())),
        None => repository
            .find_reference("refs/remotes/origin/HEAD")
            .ok()
            .and_then(|reference| reference.symbolic_target().map(str::to_string))
            .and_then(|target| repository.find_reference(target.as_str()).ok())
            .and_then(|reference| reference.target().map(|oid| oid.to_string()))
            .or_else(|| {
                [
                    "refs/remotes/origin/main".to_string(),
                    "refs/remotes/origin/master".to_string(),
                ]
                .into_iter()
                .find_map(|reference| {
                    repository
                        .find_reference(reference.as_str())
                        .ok()
                        .and_then(|reference| reference.target().map(|oid| oid.to_string()))
                })
            }),
    }
}

pub(super) fn compute_managed_drift_state(
    repository: &Repository,
    checkout_revision: Option<&str>,
    tracking_revision: Option<&str>,
    mirror_revision: Option<&str>,
) -> RepoSyncDriftState {
    let Some(checkout_revision) = checkout_revision else {
        return RepoSyncDriftState::Unknown;
    };
    let Some(mirror_revision) = mirror_revision else {
        return RepoSyncDriftState::Unknown;
    };

    if checkout_revision == mirror_revision {
        return RepoSyncDriftState::InSync;
    }

    let Some(tracking_revision) = tracking_revision else {
        return RepoSyncDriftState::Unknown;
    };

    if checkout_revision == tracking_revision {
        return RepoSyncDriftState::Behind;
    }

    if tracking_revision == mirror_revision {
        return match compare_revision_lineage(repository, checkout_revision, tracking_revision) {
            Some(Ordering::Greater) => RepoSyncDriftState::Ahead,
            Some(Ordering::Less) => RepoSyncDriftState::Behind,
            Some(Ordering::Equal) => RepoSyncDriftState::InSync,
            None => RepoSyncDriftState::Diverged,
        };
    }

    RepoSyncDriftState::Diverged
}

fn compare_revision_lineage(repository: &Repository, left: &str, right: &str) -> Option<Ordering> {
    let left = Oid::from_str(left).ok()?;
    let right = Oid::from_str(right).ok()?;

    if left == right {
        return Some(Ordering::Equal);
    }

    let left_descends = repository.graph_descendant_of(left, right).ok()?;
    let right_descends = repository.graph_descendant_of(right, left).ok()?;

    match (left_descends, right_descends) {
        (true, false) => Some(Ordering::Greater),
        (false, true) => Some(Ordering::Less),
        (false, false) => None,
        (true, true) => Some(Ordering::Equal),
    }
}

pub(super) fn discover_last_fetched_at(mirror_root: &Path) -> Option<String> {
    ["FETCH_HEAD", "HEAD"]
        .into_iter()
        .filter_map(|name| fs::metadata(mirror_root.join(name)).ok())
        .filter_map(|metadata| metadata.modified().ok())
        .max()
        .map(|modified| chrono::DateTime::<Utc>::from(modified).to_rfc3339())
}

pub(crate) fn discover_managed_remote_probe_state(
    mirror_root: &Path,
) -> Option<ManagedRemoteProbeState> {
    let payload = fs::read(mirror_root.join(MANAGED_REMOTE_PROBE_STATE_FILE)).ok()?;
    serde_json::from_slice(&payload).ok()
}

pub(crate) fn record_managed_remote_probe_state(
    mirror_root: &Path,
    target_revision: Option<&str>,
) -> std::io::Result<()> {
    let checked_at = Utc::now().to_rfc3339();
    let target_revision = target_revision.map(str::to_string);
    let payload = ManagedRemoteProbeState {
        checked_at: checked_at.clone(),
        status: ManagedRemoteProbeStatus::Success,
        target_revision: target_revision.clone(),
        error_message: None,
        last_success_checked_at: Some(checked_at),
        last_success_target_revision: target_revision,
    };
    write_managed_remote_probe_state(mirror_root, &payload)
}

pub(crate) fn record_managed_remote_probe_failure(
    mirror_root: &Path,
    error_message: &str,
    retryable: bool,
) -> std::io::Result<()> {
    let (last_success_checked_at, last_success_target_revision) =
        discover_managed_remote_probe_state(mirror_root).map_or((None, None), |state| match state
            .status
        {
            ManagedRemoteProbeStatus::Success => (Some(state.checked_at), state.target_revision),
            ManagedRemoteProbeStatus::RetryableFailure | ManagedRemoteProbeStatus::Failure => (
                state.last_success_checked_at,
                state.last_success_target_revision,
            ),
        });
    let payload = ManagedRemoteProbeState {
        checked_at: Utc::now().to_rfc3339(),
        status: if retryable {
            ManagedRemoteProbeStatus::RetryableFailure
        } else {
            ManagedRemoteProbeStatus::Failure
        },
        target_revision: None,
        error_message: Some(error_message.to_string()),
        last_success_checked_at,
        last_success_target_revision,
    };
    write_managed_remote_probe_state(mirror_root, &payload)
}

pub(crate) fn clear_managed_remote_probe_state(mirror_root: &Path) -> std::io::Result<()> {
    match fs::remove_file(mirror_root.join(MANAGED_REMOTE_PROBE_STATE_FILE)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn write_managed_remote_probe_state(
    mirror_root: &Path,
    payload: &ManagedRemoteProbeState,
) -> std::io::Result<()> {
    let payload = serde_json::to_vec(&payload)
        .map_err(|error| std::io::Error::other(format!("encode probe state: {error}")))?;
    fs::write(mirror_root.join(MANAGED_REMOTE_PROBE_STATE_FILE), payload)
}

#[cfg(test)]
mod tests {
    use super::{
        ManagedRemoteProbeStatus, clear_managed_remote_probe_state,
        discover_managed_remote_probe_state, record_managed_remote_probe_failure,
        record_managed_remote_probe_state,
    };

    fn tempdir_or_panic() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"))
    }

    fn discover_state_or_panic(mirror_root: &std::path::Path) -> super::ManagedRemoteProbeState {
        let Some(state) = discover_managed_remote_probe_state(mirror_root) else {
            panic!("expected managed remote probe state");
        };
        state
    }

    #[test]
    fn managed_remote_probe_state_round_trips() {
        let temp = tempdir_or_panic();
        record_managed_remote_probe_state(temp.path(), Some("rev-1"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));

        let state = discover_state_or_panic(temp.path());
        assert!(chrono::DateTime::parse_from_rfc3339(state.checked_at.as_str()).is_ok());
        assert_eq!(state.status, ManagedRemoteProbeStatus::Success);
        assert_eq!(state.target_revision.as_deref(), Some("rev-1"));
        assert_eq!(state.error_message, None);
        assert_eq!(
            state.last_success_checked_at.as_deref(),
            Some(state.checked_at.as_str())
        );
        assert_eq!(state.last_success_target_revision.as_deref(), Some("rev-1"));
    }

    #[test]
    fn managed_remote_probe_failure_round_trips() {
        let temp = tempdir_or_panic();
        record_managed_remote_probe_failure(temp.path(), "operation timed out", true)
            .unwrap_or_else(|error| panic!("record probe failure: {error}"));

        let state = discover_state_or_panic(temp.path());
        assert!(chrono::DateTime::parse_from_rfc3339(state.checked_at.as_str()).is_ok());
        assert_eq!(state.status, ManagedRemoteProbeStatus::RetryableFailure);
        assert_eq!(state.target_revision, None);
        assert_eq!(state.error_message.as_deref(), Some("operation timed out"));
        assert_eq!(state.last_success_checked_at, None);
        assert_eq!(state.last_success_target_revision, None);
    }

    #[test]
    fn managed_remote_probe_failure_preserves_last_success_marker() {
        let temp = tempdir_or_panic();
        record_managed_remote_probe_state(temp.path(), Some("rev-1"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));
        let success_state = discover_state_or_panic(temp.path());

        record_managed_remote_probe_failure(temp.path(), "operation timed out", true)
            .unwrap_or_else(|error| panic!("record probe failure: {error}"));

        let state = discover_state_or_panic(temp.path());
        assert_eq!(state.status, ManagedRemoteProbeStatus::RetryableFailure);
        assert_eq!(
            state.last_success_checked_at.as_deref(),
            Some(success_state.checked_at.as_str())
        );
        assert_eq!(state.last_success_target_revision.as_deref(), Some("rev-1"));
    }

    #[test]
    fn clear_managed_remote_probe_state_removes_sidecar() {
        let temp = tempdir_or_panic();
        record_managed_remote_probe_state(temp.path(), Some("rev-1"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));
        clear_managed_remote_probe_state(temp.path())
            .unwrap_or_else(|error| panic!("clear probe state: {error}"));

        assert_eq!(discover_managed_remote_probe_state(temp.path()), None);
    }
}
