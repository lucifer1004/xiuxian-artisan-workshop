use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use walkdir::WalkDir;

use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepoSyncMode, RepoSyncQuery,
    analyze_registered_repository_with_registry, repo_sync_for_registered_repository,
};

use super::types::{
    RepoCodeDocument, RepoIndexEntryStatus, RepoIndexPhase, RepoIndexSnapshot,
    RepoIndexStatusResponse,
};

#[derive(Debug, Clone)]
struct RepoIndexTask {
    repository: RegisteredRepository,
    refresh: bool,
    fingerprint: String,
}

pub(crate) struct RepoIndexCoordinator {
    project_root: PathBuf,
    plugin_registry: Arc<PluginRegistry>,
    statuses: Arc<RwLock<BTreeMap<String, RepoIndexEntryStatus>>>,
    snapshots: Arc<RwLock<BTreeMap<String, Arc<RepoIndexSnapshot>>>>,
    fingerprints: Arc<RwLock<HashMap<String, String>>>,
    queued_or_active: Arc<RwLock<HashSet<String>>>,
    current_repo_id: Arc<RwLock<Option<String>>>,
    tx: mpsc::UnboundedSender<RepoIndexTask>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<RepoIndexTask>>>,
    started: AtomicBool,
}

impl RepoIndexCoordinator {
    #[must_use]
    pub(crate) fn new(project_root: PathBuf, plugin_registry: Arc<PluginRegistry>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            project_root,
            plugin_registry,
            statuses: Arc::new(RwLock::new(BTreeMap::new())),
            snapshots: Arc::new(RwLock::new(BTreeMap::new())),
            fingerprints: Arc::new(RwLock::new(HashMap::new())),
            queued_or_active: Arc::new(RwLock::new(HashSet::new())),
            current_repo_id: Arc::new(RwLock::new(None)),
            tx,
            rx: Mutex::new(Some(rx)),
            started: AtomicBool::new(false),
        }
    }

    pub(crate) fn start(self: &Arc<Self>) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let Some(rx) = self
            .rx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        else {
            return;
        };
        if let Ok(handle) = Handle::try_current() {
            let coordinator = Arc::clone(self);
            handle.spawn(async move {
                coordinator.run(rx).await;
            });
        }
    }

    pub(crate) fn sync_repositories(&self, repositories: Vec<RegisteredRepository>) -> Vec<String> {
        let active_ids = repositories
            .iter()
            .map(|repository| repository.id.clone())
            .collect::<BTreeSet<_>>();
        self.prune_removed(&active_ids);

        let mut enqueued = Vec::new();
        for repository in repositories {
            let fingerprint = fingerprint(repository.clone());
            let existing = self
                .fingerprints
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&repository.id)
                .cloned();
            if existing.as_deref() != Some(fingerprint.as_str()) {
                self.snapshots
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&repository.id);
                if self.enqueue_repository(repository, false, true, fingerprint.clone()) {
                    enqueued.push(fingerprint_id(&fingerprint));
                }
            }
        }

        enqueued
    }

    pub(crate) fn ensure_repositories_enqueued(
        &self,
        repositories: Vec<RegisteredRepository>,
        refresh: bool,
    ) -> Vec<String> {
        let mut enqueued = Vec::new();
        for repository in repositories {
            let fingerprint = fingerprint(repository.clone());
            if self.enqueue_repository(repository, refresh, refresh, fingerprint.clone()) {
                enqueued.push(fingerprint_id(&fingerprint));
            }
        }
        enqueued
    }

    pub(crate) fn snapshot(&self, repo_id: &str) -> Option<Arc<RepoIndexSnapshot>> {
        self.snapshots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(repo_id)
            .cloned()
    }

    pub(crate) fn status_response(&self, repo_id: Option<&str>) -> RepoIndexStatusResponse {
        let statuses = self
            .statuses
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let repos = statuses
            .values()
            .filter(|status| repo_id.is_none_or(|value| value == status.repo_id))
            .cloned()
            .collect::<Vec<_>>();

        let mut response = RepoIndexStatusResponse {
            total: repos.len(),
            current_repo_id: self
                .current_repo_id
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
                .filter(|current| repo_id.is_none_or(|value| value == current)),
            repos,
            ..RepoIndexStatusResponse::default()
        };
        for status in &response.repos {
            match status.phase {
                RepoIndexPhase::Idle => {}
                RepoIndexPhase::Queued => response.queued += 1,
                RepoIndexPhase::Checking => response.checking += 1,
                RepoIndexPhase::Syncing => response.syncing += 1,
                RepoIndexPhase::Indexing => response.indexing += 1,
                RepoIndexPhase::Ready => response.ready += 1,
                RepoIndexPhase::Unsupported => response.unsupported += 1,
                RepoIndexPhase::Failed => response.failed += 1,
            }
        }
        response
    }

    fn prune_removed(&self, active_ids: &BTreeSet<String>) {
        self.statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|repo_id, _| active_ids.contains(repo_id));
        self.snapshots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|repo_id, _| active_ids.contains(repo_id));
        self.fingerprints
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|repo_id, _| active_ids.contains(repo_id));
        self.queued_or_active
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|repo_id| active_ids.contains(repo_id));
    }

    fn enqueue_repository(
        &self,
        repository: RegisteredRepository,
        refresh: bool,
        force: bool,
        fingerprint: String,
    ) -> bool {
        let repo_id = repository.id.clone();
        let existing_status = self
            .statuses
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&repo_id)
            .cloned();
        let existing_fingerprint = self
            .fingerprints
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&repo_id)
            .cloned();
        let is_same_fingerprint = existing_fingerprint.as_deref() == Some(fingerprint.as_str());

        if !force && is_same_fingerprint {
            if self
                .queued_or_active
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains(&repo_id)
            {
                return false;
            }
            if let Some(ref status) = existing_status {
                if matches!(
                    status.phase,
                    RepoIndexPhase::Ready | RepoIndexPhase::Unsupported | RepoIndexPhase::Failed
                ) {
                    return false;
                }
            }
        }

        self.fingerprints
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(repo_id.clone(), fingerprint.clone());
        self.queued_or_active
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(repo_id.clone());
        self.statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                repo_id.clone(),
                RepoIndexEntryStatus {
                    repo_id: repo_id.clone(),
                    phase: RepoIndexPhase::Queued,
                    last_error: None,
                    last_revision: existing_status
                        .as_ref()
                        .and_then(|status| status.last_revision.clone()),
                    updated_at: Some(timestamp_now()),
                    attempt_count: 0,
                },
            );
        self.tx
            .send(RepoIndexTask {
                repository,
                refresh,
                fingerprint,
            })
            .is_ok()
    }

    async fn run(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<RepoIndexTask>) {
        while let Some(task) = rx.recv().await {
            let repo_id = task.repository.id.clone();
            let current_fingerprint = self
                .fingerprints
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&repo_id)
                .cloned();
            if current_fingerprint.as_deref() != Some(task.fingerprint.as_str()) {
                self.queued_or_active
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&repo_id);
                continue;
            }

            *self
                .current_repo_id
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(repo_id.clone());
            self.bump_status(&repo_id, RepoIndexPhase::Checking, None, None);
            self.bump_status(&repo_id, RepoIndexPhase::Syncing, None, None);

            let sync_result = repo_sync_for_registered_repository(
                &RepoSyncQuery {
                    repo_id: repo_id.clone(),
                    mode: if task.refresh {
                        RepoSyncMode::Refresh
                    } else {
                        RepoSyncMode::Ensure
                    },
                },
                &task.repository,
                self.project_root.as_path(),
            );
            let sync_result = match sync_result {
                Ok(result) => result,
                Err(error) => {
                    self.finish_error(&repo_id, error, None);
                    continue;
                }
            };

            self.bump_status(
                &repo_id,
                RepoIndexPhase::Indexing,
                sync_result.revision.clone(),
                None,
            );

            let analysis = analyze_registered_repository_with_registry(
                &task.repository,
                self.project_root.as_path(),
                &self.plugin_registry,
            );
            match analysis {
                Ok(analysis) => {
                    let snapshot = Arc::new(RepoIndexSnapshot {
                        repo_id: repo_id.clone(),
                        code_documents: Arc::new(collect_code_documents(Path::new(
                            sync_result.checkout_path.as_str(),
                        ))),
                        analysis: Arc::new(analysis),
                    });
                    self.snapshots
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(repo_id.clone(), snapshot);
                    self.statuses
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(
                            repo_id.clone(),
                            RepoIndexEntryStatus {
                                repo_id: repo_id.clone(),
                                phase: RepoIndexPhase::Ready,
                                last_error: None,
                                last_revision: sync_result.revision,
                                updated_at: Some(timestamp_now()),
                                attempt_count: self.next_attempt_count(&repo_id),
                            },
                        );
                }
                Err(error) => {
                    self.finish_error(&repo_id, error, sync_result.revision);
                }
            }

            self.queued_or_active
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&repo_id);
            *self
                .current_repo_id
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
    }

    fn bump_status(
        &self,
        repo_id: &str,
        phase: RepoIndexPhase,
        last_revision: Option<String>,
        last_error: Option<String>,
    ) {
        let attempt_count = self.next_attempt_count(repo_id);
        self.statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                repo_id.to_string(),
                RepoIndexEntryStatus {
                    repo_id: repo_id.to_string(),
                    phase,
                    last_error,
                    last_revision,
                    updated_at: Some(timestamp_now()),
                    attempt_count,
                },
            );
    }

    fn finish_error(
        &self,
        repo_id: &str,
        error: RepoIntelligenceError,
        last_revision: Option<String>,
    ) {
        self.snapshots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(repo_id);
        let phase = if matches!(
            error,
            RepoIntelligenceError::UnsupportedRepositoryLayout { .. }
        ) {
            RepoIndexPhase::Unsupported
        } else {
            RepoIndexPhase::Failed
        };
        self.statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                repo_id.to_string(),
                RepoIndexEntryStatus {
                    repo_id: repo_id.to_string(),
                    phase,
                    last_error: Some(error.to_string()),
                    last_revision,
                    updated_at: Some(timestamp_now()),
                    attempt_count: self.next_attempt_count(repo_id),
                },
            );
        self.queued_or_active
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(repo_id);
        *self
            .current_repo_id
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    fn next_attempt_count(&self, repo_id: &str) -> usize {
        self.statuses
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(repo_id)
            .map_or(1, |status| status.attempt_count.saturating_add(1))
    }

    #[cfg(test)]
    pub(crate) fn set_snapshot_for_test(&self, snapshot: Arc<RepoIndexSnapshot>) {
        self.snapshots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(snapshot.repo_id.clone(), snapshot);
    }

    #[cfg(test)]
    pub(crate) fn set_status_for_test(&self, status: RepoIndexEntryStatus) {
        self.statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(status.repo_id.clone(), status);
    }
}

fn fingerprint(repository: RegisteredRepository) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        repository.id,
        repository.path,
        repository.url,
        repository.git_ref,
        repository.refresh,
        repository.plugins
    )
}

fn fingerprint_id(fingerprint: &str) -> String {
    fingerprint
        .split('|')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn timestamp_now() -> String {
    Utc::now().to_rfc3339()
}

fn collect_code_documents(root: &Path) -> Vec<RepoCodeDocument> {
    let mut documents = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let relative_path = entry
            .path()
            .strip_prefix(root)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| entry.path().to_string_lossy().replace('\\', "/"));
        if is_excluded_code_path(relative_path.as_str())
            || !is_supported_code_path(relative_path.as_str())
        {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        documents.push(RepoCodeDocument {
            language: infer_code_language(relative_path.as_str()),
            path: relative_path,
            contents: Arc::<str>::from(contents),
        });
    }
    documents
}

fn is_supported_code_path(path: &str) -> bool {
    path.ends_with(".jl")
        || path.ends_with(".julia")
        || path.ends_with(".mo")
        || path.ends_with(".modelica")
}

fn is_excluded_code_path(path: &str) -> bool {
    [
        ".git/",
        ".cache/",
        ".devenv/",
        ".direnv/",
        "node_modules/",
        "target/",
        "dist/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn infer_code_language(path: &str) -> Option<String> {
    if path.ends_with(".jl") || path.ends_with(".julia") {
        return Some("julia".to_string());
    }
    if path.ends_with(".mo") || path.ends_with(".modelica") {
        return Some("modelica".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::RepositoryPluginConfig;

    fn repo(id: &str, path: &str) -> RegisteredRepository {
        RegisteredRepository {
            id: id.to_string(),
            path: Some(PathBuf::from(path)),
            url: None,
            git_ref: None,
            refresh: crate::analyzers::RepositoryRefreshPolicy::Fetch,
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
        }
    }

    #[test]
    fn sync_repositories_only_enqueues_new_or_changed_repositories() {
        let coordinator =
            RepoIndexCoordinator::new(PathBuf::from("."), Arc::new(PluginRegistry::new()));

        let first = coordinator.sync_repositories(vec![repo("sciml", "./sciml")]);
        let second = coordinator.sync_repositories(vec![repo("sciml", "./sciml")]);
        let third = coordinator.sync_repositories(vec![repo("sciml", "./sciml-next")]);

        assert_eq!(first, vec!["sciml".to_string()]);
        assert!(second.is_empty());
        assert_eq!(third, vec!["sciml".to_string()]);
    }

    #[test]
    fn status_response_counts_each_phase() {
        let coordinator =
            RepoIndexCoordinator::new(PathBuf::from("."), Arc::new(PluginRegistry::new()));
        coordinator
            .statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend([
                (
                    "queued".to_string(),
                    RepoIndexEntryStatus {
                        repo_id: "queued".to_string(),
                        phase: RepoIndexPhase::Queued,
                        last_error: None,
                        last_revision: None,
                        updated_at: Some(timestamp_now()),
                        attempt_count: 1,
                    },
                ),
                (
                    "ready".to_string(),
                    RepoIndexEntryStatus {
                        repo_id: "ready".to_string(),
                        phase: RepoIndexPhase::Ready,
                        last_error: None,
                        last_revision: None,
                        updated_at: Some(timestamp_now()),
                        attempt_count: 1,
                    },
                ),
            ]);

        let status = coordinator.status_response(None);
        assert_eq!(status.total, 2);
        assert_eq!(status.queued, 1);
        assert_eq!(status.ready, 1);
    }
}
