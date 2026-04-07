use std::path::Path;

use chrono::Utc;

use crate::analyzers::query::{
    RepoSourceKind, RepoSyncHealthState, RepoSyncMode, RepoSyncResult, RepoSyncStalenessState,
};
use crate::analyzers::{
    RegisteredRepository, RepositoryRef, RepositoryRefreshPolicy,
    repo_sync_for_registered_repository,
};
use crate::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::repo_index::state::fingerprint::fingerprint;
use crate::repo_index::types::{RepoIndexEntryStatus, RepoIndexPhase};
use xiuxian_git_repo::{ManagedRemoteProbeStatus, discover_managed_remote_probe_state};

impl RepoIndexCoordinator {
    pub(crate) fn hydrate_repositories_from_search_plane(
        &self,
        repositories: &[RegisteredRepository],
    ) {
        let queued_or_active = self
            .queued_or_active
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let existing_fingerprints = self
            .fingerprints
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let repo_ids = repositories
            .iter()
            .filter(|repository| !queued_or_active.contains(&repository.id))
            .filter(|repository| !existing_fingerprints.contains_key(&repository.id))
            .map(|repository| repository.id.clone())
            .collect::<Vec<_>>();
        if repo_ids.is_empty() {
            return;
        }

        let bootstrap_statuses = self.search_plane.repo_index_bootstrap_statuses(&repo_ids);
        if bootstrap_statuses.is_empty() {
            return;
        }

        let mut statuses = self
            .statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut fingerprints = self
            .fingerprints
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut status_changed = false;

        for repository in repositories {
            if queued_or_active.contains(&repository.id)
                || fingerprints.contains_key(&repository.id)
            {
                continue;
            }
            let Some(status) = bootstrap_statuses.get(&repository.id).cloned() else {
                continue;
            };
            if !self.repository_is_safe_to_hydrate(repository, &status) {
                continue;
            }

            fingerprints.insert(repository.id.clone(), fingerprint(repository));
            if statuses.get(&repository.id) != Some(&status) {
                statuses.insert(repository.id.clone(), status);
                status_changed = true;
            }
        }
        drop(fingerprints);
        drop(statuses);

        if status_changed {
            self.refresh_status_snapshot();
        }
    }

    fn repository_is_safe_to_hydrate(
        &self,
        repository: &RegisteredRepository,
        status: &RepoIndexEntryStatus,
    ) -> bool {
        if !persisted_publication_bootstrap_is_searchable(status) {
            return false;
        }
        if repository.url.is_none() {
            return true;
        }

        let Some(sync_result) = self.managed_remote_status(repository) else {
            return true;
        };
        managed_remote_bootstrap_is_safe(
            repository,
            status,
            &sync_result,
            managed_remote_probe_freshness(&sync_result),
            managed_remote_retryable_probe_failure_is_recent(&sync_result),
        ) || persisted_publication_bootstrap_is_searchable(status)
    }

    fn managed_remote_status(&self, repository: &RegisteredRepository) -> Option<RepoSyncResult> {
        repo_sync_for_registered_repository(
            &crate::analyzers::RepoSyncQuery {
                repo_id: repository.id.clone(),
                mode: RepoSyncMode::Status,
            },
            repository,
            self.project_root.as_path(),
        )
        .ok()
    }
}

fn persisted_publication_bootstrap_is_searchable(status: &RepoIndexEntryStatus) -> bool {
    matches!(status.phase, RepoIndexPhase::Ready)
}

fn managed_remote_bootstrap_is_safe(
    repository: &RegisteredRepository,
    status: &RepoIndexEntryStatus,
    sync_result: &RepoSyncResult,
    probe_freshness: Option<RepoSyncStalenessState>,
    recent_retryable_probe_failure: bool,
) -> bool {
    if !matches!(status.phase, RepoIndexPhase::Ready) {
        return false;
    }
    if sync_result.source_kind != RepoSourceKind::ManagedRemote {
        return false;
    }
    if sync_result.health_state != RepoSyncHealthState::Healthy {
        return false;
    }
    if sync_result.revision.as_deref() != status.last_revision.as_deref() {
        return false;
    }

    match repository.refresh {
        RepositoryRefreshPolicy::Manual => true,
        RepositoryRefreshPolicy::Fetch
            if matches!(repository.git_ref, Some(RepositoryRef::Commit(_))) =>
        {
            true
        }
        RepositoryRefreshPolicy::Fetch => {
            matches!(
                sync_result.staleness_state,
                RepoSyncStalenessState::Fresh | RepoSyncStalenessState::Aging
            ) || matches!(
                probe_freshness,
                Some(RepoSyncStalenessState::Fresh | RepoSyncStalenessState::Aging)
            ) || recent_retryable_probe_failure
        }
    }
}

fn managed_remote_probe_freshness(sync_result: &RepoSyncResult) -> Option<RepoSyncStalenessState> {
    let mirror_root = Path::new(sync_result.mirror_path.as_deref()?);
    let probe_state = discover_managed_remote_probe_state(mirror_root)?;
    let success_checked_at = if probe_state.status == ManagedRemoteProbeStatus::Success {
        Some(probe_state.checked_at.as_str())
    } else {
        probe_state.last_success_checked_at.as_deref()
    }?;
    let success_target_revision = if probe_state.status == ManagedRemoteProbeStatus::Success {
        probe_state.target_revision.as_deref()
    } else {
        probe_state.last_success_target_revision.as_deref()
    };
    if success_target_revision != sync_result.revision.as_deref() {
        return None;
    }
    probe_staleness_state(success_checked_at, Utc::now())
}

fn managed_remote_retryable_probe_failure_is_recent(sync_result: &RepoSyncResult) -> bool {
    let Some(mirror_path) = sync_result.mirror_path.as_deref() else {
        return false;
    };
    let Some(probe_state) = discover_managed_remote_probe_state(Path::new(mirror_path)) else {
        return false;
    };
    if probe_state.status != ManagedRemoteProbeStatus::RetryableFailure {
        return false;
    }
    matches!(
        probe_staleness_state(probe_state.checked_at.as_str(), Utc::now()),
        Some(RepoSyncStalenessState::Fresh)
    )
}

fn probe_staleness_state(
    checked_at: &str,
    now: chrono::DateTime<Utc>,
) -> Option<RepoSyncStalenessState> {
    let checked_at = chrono::DateTime::parse_from_rfc3339(checked_at)
        .ok()?
        .with_timezone(&Utc);
    let age = now.signed_duration_since(checked_at);
    if age < chrono::Duration::zero() {
        return None;
    }
    if age < chrono::Duration::hours(1) {
        return Some(RepoSyncStalenessState::Fresh);
    }
    if age < chrono::Duration::hours(24) {
        return Some(RepoSyncStalenessState::Aging);
    }
    Some(RepoSyncStalenessState::Stale)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        managed_remote_bootstrap_is_safe, managed_remote_probe_freshness,
        managed_remote_retryable_probe_failure_is_recent,
    };
    use crate::analyzers::query::{
        RepoSourceKind, RepoSyncHealthState, RepoSyncResult, RepoSyncStalenessState,
    };
    use crate::analyzers::{RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy};
    use crate::repo_index::types::{RepoIndexEntryStatus, RepoIndexPhase};
    use xiuxian_git_repo::{
        record_managed_remote_probe_failure, record_managed_remote_probe_state,
    };

    fn tempdir_or_panic() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"))
    }

    fn read_json_value_or_panic(path: &std::path::Path) -> serde_json::Value {
        let payload = fs::read(path).unwrap_or_else(|error| panic!("read probe state: {error}"));
        serde_json::from_slice(&payload)
            .unwrap_or_else(|error| panic!("parse probe state: {error}"))
    }

    fn write_json_value_or_panic(path: &std::path::Path, payload: &serde_json::Value) {
        let encoded = serde_json::to_vec(payload)
            .unwrap_or_else(|error| panic!("encode probe state: {error}"));
        fs::write(path, encoded).unwrap_or_else(|error| panic!("rewrite probe state: {error}"));
    }

    #[test]
    fn managed_remote_bootstrap_requires_ready_status_and_matching_revision() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        let mut sync_result = managed_remote_sync_result();

        let ready = ready_status(Some("rev-1"));
        assert!(managed_remote_bootstrap_is_safe(
            &repository,
            &ready,
            &sync_result,
            None,
            false,
        ));

        sync_result.revision = Some("rev-2".to_string());
        assert!(!managed_remote_bootstrap_is_safe(
            &repository,
            &ready,
            &sync_result,
            None,
            false,
        ));

        assert!(!managed_remote_bootstrap_is_safe(
            &repository,
            &RepoIndexEntryStatus {
                phase: RepoIndexPhase::Failed,
                ..ready
            },
            &managed_remote_sync_result(),
            None,
            false,
        ));
    }

    #[test]
    fn persisted_publication_bootstrap_only_accepts_ready_status() {
        assert!(super::persisted_publication_bootstrap_is_searchable(
            &ready_status(None)
        ));
        assert!(!super::persisted_publication_bootstrap_is_searchable(
            &RepoIndexEntryStatus {
                phase: RepoIndexPhase::Queued,
                ..ready_status(None)
            }
        ));
    }

    #[test]
    fn managed_remote_bootstrap_allows_aging_fetch_for_fetch_policy() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Aging;

        assert!(managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            None,
            false,
        ));
    }

    #[test]
    fn managed_remote_bootstrap_rejects_stale_fetch_for_fetch_policy() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Stale;

        assert!(!managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            None,
            false,
        ));
    }

    #[test]
    fn managed_remote_bootstrap_allows_manual_policy_without_fresh_fetch() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Manual);
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Stale;

        assert!(managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            None,
            false,
        ));
    }

    #[test]
    fn managed_remote_bootstrap_allows_stale_commit_pinned_fetch_policy() {
        let mut repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        repository.git_ref = Some(crate::analyzers::RepositoryRef::Commit("rev-1".to_string()));
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Stale;

        assert!(managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            None,
            false,
        ));
    }

    #[test]
    fn managed_remote_bootstrap_allows_stale_fetch_policy_when_probe_state_is_recent() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Stale;

        let probe_dir = tempdir_or_panic();
        record_managed_remote_probe_state(probe_dir.path(), Some("rev-1"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));
        sync_result.mirror_path = Some(probe_dir.path().display().to_string());

        assert!(managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            managed_remote_probe_freshness(&sync_result),
            false,
        ));
    }

    #[test]
    fn managed_remote_probe_freshness_ignores_mismatched_revision() {
        let mut sync_result = managed_remote_sync_result();
        let probe_dir = tempdir_or_panic();
        record_managed_remote_probe_state(probe_dir.path(), Some("rev-2"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));
        sync_result.mirror_path = Some(probe_dir.path().display().to_string());

        assert_eq!(managed_remote_probe_freshness(&sync_result), None);
    }

    #[test]
    fn managed_remote_probe_freshness_reports_stale_for_old_probe_state() {
        let mut sync_result = managed_remote_sync_result();
        let probe_dir = tempdir_or_panic();
        record_managed_remote_probe_state(probe_dir.path(), Some("rev-1"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));
        let state_path = probe_dir.path().join("xiuxian-upstream-probe-state.json");
        let mut payload = read_json_value_or_panic(&state_path);
        payload["checked_at"] = serde_json::Value::String("2000-01-01T00:00:00+00:00".to_string());
        write_json_value_or_panic(&state_path, &payload);
        sync_result.mirror_path = Some(probe_dir.path().display().to_string());

        assert_eq!(
            managed_remote_probe_freshness(&sync_result),
            Some(RepoSyncStalenessState::Stale)
        );
    }

    #[test]
    fn managed_remote_bootstrap_allows_recent_retryable_probe_failure() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Stale;

        let probe_dir = tempdir_or_panic();
        record_managed_remote_probe_failure(probe_dir.path(), "operation timed out", true)
            .unwrap_or_else(|error| panic!("record probe failure: {error}"));
        sync_result.mirror_path = Some(probe_dir.path().display().to_string());

        assert!(managed_remote_retryable_probe_failure_is_recent(
            &sync_result
        ));
        assert!(managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            None,
            true,
        ));
    }

    #[test]
    fn managed_remote_bootstrap_rejects_non_retryable_probe_failure() {
        let repository = managed_remote_repository(RepositoryRefreshPolicy::Fetch);
        let mut sync_result = managed_remote_sync_result();
        sync_result.staleness_state = RepoSyncStalenessState::Stale;

        let probe_dir = tempdir_or_panic();
        record_managed_remote_probe_failure(probe_dir.path(), "authentication required", false)
            .unwrap_or_else(|error| panic!("record probe failure: {error}"));
        sync_result.mirror_path = Some(probe_dir.path().display().to_string());

        assert!(!managed_remote_retryable_probe_failure_is_recent(
            &sync_result
        ));
        assert!(!managed_remote_bootstrap_is_safe(
            &repository,
            &ready_status(Some("rev-1")),
            &sync_result,
            None,
            false,
        ));
    }

    #[test]
    fn managed_remote_probe_freshness_uses_last_success_marker_after_retryable_failure() {
        let mut sync_result = managed_remote_sync_result();
        let probe_dir = tempdir_or_panic();
        record_managed_remote_probe_state(probe_dir.path(), Some("rev-1"))
            .unwrap_or_else(|error| panic!("record probe state: {error}"));
        record_managed_remote_probe_failure(probe_dir.path(), "operation timed out", true)
            .unwrap_or_else(|error| panic!("record probe failure: {error}"));
        sync_result.mirror_path = Some(probe_dir.path().display().to_string());

        assert!(matches!(
            managed_remote_probe_freshness(&sync_result),
            Some(RepoSyncStalenessState::Fresh)
        ));
    }

    fn managed_remote_repository(refresh: RepositoryRefreshPolicy) -> RegisteredRepository {
        RegisteredRepository {
            id: "managed-remote".to_string(),
            path: None,
            url: Some("https://example.com/managed-remote.git".to_string()),
            git_ref: None,
            refresh,
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
        }
    }

    fn ready_status(last_revision: Option<&str>) -> RepoIndexEntryStatus {
        RepoIndexEntryStatus {
            repo_id: "managed-remote".to_string(),
            phase: RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: last_revision.map(str::to_string),
            updated_at: Some("2026-04-02T00:00:00Z".to_string()),
            attempt_count: 0,
        }
    }

    fn managed_remote_sync_result() -> RepoSyncResult {
        RepoSyncResult {
            repo_id: "managed-remote".to_string(),
            source_kind: RepoSourceKind::ManagedRemote,
            health_state: RepoSyncHealthState::Healthy,
            staleness_state: RepoSyncStalenessState::Fresh,
            revision: Some("rev-1".to_string()),
            ..RepoSyncResult::default()
        }
    }
}
