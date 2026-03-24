use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;

use super::{
    SearchCorpusKind, SearchCorpusStatus, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlanePhase, SearchPlaneStatusSnapshot,
};

/// Reason that triggered a background compaction request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchCompactionReason {
    /// Publish count crossed the maintenance threshold.
    PublishThreshold,
    /// Row-count drift crossed the maintenance threshold.
    RowDeltaRatio,
}

impl SearchCompactionReason {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PublishThreshold => "publish_threshold",
            Self::RowDeltaRatio => "row_delta_ratio",
        }
    }
}

/// Pending compaction task derived from current corpus state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchCompactionTask {
    /// Corpus whose active epoch should be compacted.
    pub corpus: SearchCorpusKind,
    /// Active epoch to compact.
    pub active_epoch: u64,
    /// Published row count for the active epoch.
    pub row_count: u64,
    /// Policy reason that triggered compaction.
    pub reason: SearchCompactionReason,
}

/// Single build token for one corpus epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchBuildLease {
    /// Corpus being built.
    pub corpus: SearchCorpusKind,
    /// Fingerprint bound to the in-flight build.
    pub fingerprint: String,
    /// Staging epoch assigned to this build.
    pub epoch: u64,
    /// Schema version expected by this build.
    pub schema_version: u32,
}

/// Result of attempting to start a background build.
#[derive(Debug, Clone, PartialEq)]
pub enum BeginBuildDecision {
    /// A new staging build has been leased.
    Started(SearchBuildLease),
    /// The requested fingerprint is already published and ready.
    AlreadyReady(SearchCorpusStatus),
    /// The requested fingerprint is already being indexed.
    AlreadyIndexing(SearchCorpusStatus),
}

#[derive(Debug, Clone)]
struct SearchCorpusRuntime {
    status: SearchCorpusStatus,
    next_epoch: u64,
    last_compacted_row_count: Option<u64>,
}

impl SearchCorpusRuntime {
    fn new(corpus: SearchCorpusKind) -> Self {
        Self {
            status: SearchCorpusStatus::new(corpus),
            next_epoch: 1,
            last_compacted_row_count: None,
        }
    }
}

/// In-memory, per-corpus build coordinator for the Studio search plane.
pub struct SearchPlaneCoordinator {
    project_root: PathBuf,
    storage_root: PathBuf,
    manifest_keyspace: SearchManifestKeyspace,
    maintenance_policy: SearchMaintenancePolicy,
    state: Arc<RwLock<BTreeMap<SearchCorpusKind, SearchCorpusRuntime>>>,
    spawn_lock: Mutex<()>,
}

impl SearchPlaneCoordinator {
    /// Construct a coordinator for one project-local search plane.
    #[must_use]
    pub fn new(
        project_root: PathBuf,
        storage_root: PathBuf,
        manifest_keyspace: SearchManifestKeyspace,
        maintenance_policy: SearchMaintenancePolicy,
    ) -> Self {
        let state = SearchCorpusKind::ALL
            .into_iter()
            .map(|corpus| (corpus, SearchCorpusRuntime::new(corpus)))
            .collect();
        Self {
            project_root,
            storage_root,
            manifest_keyspace,
            maintenance_policy,
            state: Arc::new(RwLock::new(state)),
            spawn_lock: Mutex::new(()),
        }
    }

    /// Absolute project root associated with this coordinator.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Root directory that stores per-corpus Lance datasets.
    #[must_use]
    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    /// Valkey namespace used for manifests, leases, and short-lived caches.
    #[must_use]
    pub fn manifest_keyspace(&self) -> &SearchManifestKeyspace {
        &self.manifest_keyspace
    }

    /// Maintenance policy that decides when compaction should run.
    #[must_use]
    pub fn maintenance_policy(&self) -> &SearchMaintenancePolicy {
        &self.maintenance_policy
    }

    /// Snapshot ordered status rows for every corpus.
    #[must_use]
    pub fn status(&self) -> SearchPlaneStatusSnapshot {
        let state = self
            .state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let corpora = SearchCorpusKind::ALL
            .iter()
            .filter_map(|corpus| state.get(corpus).map(|runtime| runtime.status.clone()))
            .collect();
        SearchPlaneStatusSnapshot { corpora }
    }

    /// Read the current status for one corpus.
    #[must_use]
    pub fn status_for(&self, corpus: SearchCorpusKind) -> SearchCorpusStatus {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&corpus)
            .map_or_else(
                || SearchCorpusStatus::new(corpus),
                |runtime| runtime.status.clone(),
            )
    }

    /// Replace the runtime status for a corpus from an external publisher.
    pub fn replace_status(&self, status: SearchCorpusStatus) {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let runtime = state
            .entry(status.corpus)
            .or_insert_with(|| SearchCorpusRuntime::new(status.corpus));
        runtime.next_epoch = runtime
            .next_epoch
            .max(status.active_epoch.unwrap_or_default().saturating_add(1))
            .max(status.staging_epoch.unwrap_or_default().saturating_add(1));
        runtime.status = status;
    }

    /// Attempt to start a new staging build for a corpus fingerprint.
    pub fn begin_build(
        &self,
        corpus: SearchCorpusKind,
        fingerprint: impl Into<String>,
        schema_version: u32,
    ) -> BeginBuildDecision {
        let _spawn_guard = self
            .spawn_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fingerprint = fingerprint.into();
        let now = timestamp_now();
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let runtime = state
            .entry(corpus)
            .or_insert_with(|| SearchCorpusRuntime::new(corpus));
        let schema_matches = runtime.status.schema_version == schema_version;
        if runtime.status.fingerprint.as_deref() == Some(fingerprint.as_str()) && schema_matches {
            if matches!(runtime.status.phase, SearchPlanePhase::Ready)
                && runtime.status.active_epoch.is_some()
            {
                return BeginBuildDecision::AlreadyReady(runtime.status.clone());
            }
            if matches!(runtime.status.phase, SearchPlanePhase::Indexing) {
                return BeginBuildDecision::AlreadyIndexing(runtime.status.clone());
            }
        }

        let epoch = runtime.next_epoch;
        runtime.next_epoch = runtime.next_epoch.saturating_add(1);
        runtime.status.phase = SearchPlanePhase::Indexing;
        runtime.status.staging_epoch = Some(epoch);
        runtime.status.schema_version = schema_version;
        runtime.status.fingerprint = Some(fingerprint.clone());
        runtime.status.progress = Some(0.0);
        runtime.status.build_started_at = Some(now.clone());
        runtime.status.build_finished_at = None;
        runtime.status.updated_at = Some(now);
        runtime.status.last_error = None;

        BeginBuildDecision::Started(SearchBuildLease {
            corpus,
            fingerprint,
            epoch,
            schema_version,
        })
    }

    /// Update build progress for a live lease.
    pub fn update_progress(&self, lease: &SearchBuildLease, progress: f32) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&lease.corpus) else {
            return false;
        };
        if !matches!(runtime.status.phase, SearchPlanePhase::Indexing)
            || runtime.status.staging_epoch != Some(lease.epoch)
            || runtime.status.fingerprint.as_deref() != Some(lease.fingerprint.as_str())
        {
            return false;
        }
        runtime.status.progress = Some(progress.clamp(0.0, 1.0));
        runtime.status.updated_at = Some(timestamp_now());
        true
    }

    /// Publish a completed staging epoch if the lease is still current.
    pub fn publish_ready(
        &self,
        lease: &SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&lease.corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(lease.epoch)
            || runtime.status.fingerprint.as_deref() != Some(lease.fingerprint.as_str())
        {
            return false;
        }

        let now = timestamp_now();
        let publish_count = runtime
            .status
            .maintenance
            .publish_count_since_compaction
            .saturating_add(1);
        runtime.status.phase = SearchPlanePhase::Ready;
        runtime.status.active_epoch = Some(lease.epoch);
        runtime.status.staging_epoch = None;
        runtime.status.schema_version = lease.schema_version;
        runtime.status.progress = None;
        runtime.status.row_count = Some(row_count);
        runtime.status.fragment_count = Some(fragment_count);
        runtime.status.build_finished_at = Some(now.clone());
        runtime.status.updated_at = Some(now);
        runtime.status.last_error = None;
        runtime.status.maintenance.publish_count_since_compaction = publish_count;
        runtime.status.maintenance.compaction_pending = self.maintenance_policy.should_compact(
            publish_count,
            runtime.last_compacted_row_count,
            row_count,
        );
        true
    }

    /// Mark an in-flight build as failed if the lease is still current.
    pub fn fail_build(&self, lease: &SearchBuildLease, error: impl Into<String>) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&lease.corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(lease.epoch)
            || runtime.status.fingerprint.as_deref() != Some(lease.fingerprint.as_str())
        {
            return false;
        }

        let now = timestamp_now();
        runtime.status.phase = SearchPlanePhase::Failed;
        runtime.status.staging_epoch = None;
        runtime.status.progress = None;
        runtime.status.build_finished_at = Some(now.clone());
        runtime.status.updated_at = Some(now);
        runtime.status.last_error = Some(error.into());
        true
    }

    /// Record that compaction completed for the currently active epoch.
    pub(crate) fn mark_compaction_complete(
        &self,
        corpus: SearchCorpusKind,
        active_epoch: u64,
        row_count: u64,
        fragment_count: u64,
        reason: SearchCompactionReason,
    ) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&corpus) else {
            return false;
        };
        if runtime.status.active_epoch != Some(active_epoch) {
            return false;
        }

        runtime.last_compacted_row_count = Some(row_count);
        runtime.status.fragment_count = Some(fragment_count);
        runtime.status.maintenance.compaction_pending = false;
        runtime.status.maintenance.publish_count_since_compaction = 0;
        runtime.status.maintenance.last_compacted_at = Some(timestamp_now());
        runtime.status.maintenance.last_compaction_reason = Some(reason.as_str().to_string());
        runtime.status.updated_at = runtime.status.maintenance.last_compacted_at.clone();
        true
    }

    /// Return the current compaction task for a ready corpus, if maintenance is pending.
    #[must_use]
    pub(crate) fn pending_compaction_task(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchCompactionTask> {
        let state = self
            .state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let runtime = state.get(&corpus)?;
        if !matches!(runtime.status.phase, SearchPlanePhase::Ready)
            || !runtime.status.maintenance.compaction_pending
        {
            return None;
        }
        let active_epoch = runtime.status.active_epoch?;
        let row_count = runtime.status.row_count?;
        let reason = self.maintenance_policy.compaction_reason(
            runtime.status.maintenance.publish_count_since_compaction,
            runtime.last_compacted_row_count,
            row_count,
        )?;
        Some(SearchCompactionTask {
            corpus,
            active_epoch,
            row_count,
            reason,
        })
    }
}

fn timestamp_now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::search_plane::SearchManifestKeyspace;

    fn coordinator_with_policy(policy: SearchMaintenancePolicy) -> SearchPlaneCoordinator {
        SearchPlaneCoordinator::new(
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/storage"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            policy,
        )
    }

    #[test]
    fn begin_build_marks_corpus_as_indexing() {
        let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
        let decision = coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 3);
        let BeginBuildDecision::Started(lease) = decision else {
            panic!("expected started build lease");
        };

        let status = coordinator.status_for(SearchCorpusKind::LocalSymbol);
        assert_eq!(status.phase, SearchPlanePhase::Indexing);
        assert_eq!(status.staging_epoch, Some(lease.epoch));
        assert_eq!(status.schema_version, 3);
        assert_eq!(status.fingerprint.as_deref(), Some("fingerprint-a"));
        assert_eq!(status.progress, Some(0.0));
    }

    #[test]
    fn stale_publish_is_discarded_after_newer_build_starts() {
        let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
        let old = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 1) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin result: {other:?}"),
        };
        let next = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-b", 1)
        {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin result: {other:?}"),
        };

        assert!(!coordinator.publish_ready(&old, 10, 2));
        assert!(coordinator.publish_ready(&next, 20, 4));

        let status = coordinator.status_for(SearchCorpusKind::LocalSymbol);
        assert_eq!(status.phase, SearchPlanePhase::Ready);
        assert_eq!(status.active_epoch, Some(next.epoch));
        assert_eq!(status.row_count, Some(20));
    }

    #[test]
    fn schema_version_mismatch_forces_rebuild_even_with_same_fingerprint() {
        let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
        let old = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 1) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert!(coordinator.publish_ready(&old, 10, 2));

        let next = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 2)
        {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("expected rebuild for schema mismatch, got {other:?}"),
        };

        assert_ne!(next.epoch, old.epoch);
        let status = coordinator.status_for(SearchCorpusKind::LocalSymbol);
        assert_eq!(status.phase, SearchPlanePhase::Indexing);
        assert_eq!(status.schema_version, 2);
        assert_eq!(status.staging_epoch, Some(next.epoch));
    }

    #[test]
    fn maintenance_policy_marks_compaction_pending_after_threshold() {
        let coordinator = coordinator_with_policy(SearchMaintenancePolicy {
            publish_count_threshold: 2,
            row_delta_ratio_threshold: 0.90,
        });
        let first = match coordinator.begin_build(SearchCorpusKind::RepoEntity, "fp-1", 1) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert!(coordinator.publish_ready(&first, 100, 5));
        assert!(
            !coordinator
                .status_for(SearchCorpusKind::RepoEntity)
                .maintenance
                .compaction_pending
        );

        let second = match coordinator.begin_build(SearchCorpusKind::RepoEntity, "fp-2", 1) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert!(coordinator.publish_ready(&second, 110, 6));
        assert!(
            coordinator
                .status_for(SearchCorpusKind::RepoEntity)
                .maintenance
                .compaction_pending
        );

        assert!(coordinator.mark_compaction_complete(
            SearchCorpusKind::RepoEntity,
            second.epoch,
            110,
            2,
            SearchCompactionReason::PublishThreshold
        ));
        let status = coordinator.status_for(SearchCorpusKind::RepoEntity);
        assert!(!status.maintenance.compaction_pending);
        assert_eq!(status.maintenance.publish_count_since_compaction, 0);
        assert_eq!(
            status.maintenance.last_compaction_reason.as_deref(),
            Some("publish_threshold")
        );
    }

    #[test]
    fn replace_status_updates_runtime_snapshot() {
        let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
        let mut status = SearchCorpusStatus::new(SearchCorpusKind::RepoEntity);
        status.phase = SearchPlanePhase::Ready;
        status.active_epoch = Some(77);
        status.staging_epoch = Some(79);
        status.row_count = Some(42);
        status.fingerprint = Some("repo-fingerprint".to_string());

        coordinator.replace_status(status.clone());

        assert_eq!(coordinator.status_for(SearchCorpusKind::RepoEntity), status);
        let snapshot = coordinator.status();
        let stored = snapshot
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            .unwrap_or_else(|| panic!("repo entity status should be present"));
        assert_eq!(stored.active_epoch, Some(77));
        assert_eq!(stored.staging_epoch, Some(79));
        assert_eq!(stored.fingerprint.as_deref(), Some("repo-fingerprint"));
    }
}
