use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use tokio::runtime::Handle;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};
use tokio::task::{JoinHandle, JoinSet};

use crate::analyzers::query::RepoSyncResult;
use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepoSyncMode, RepoSyncQuery,
    analyze_registered_repository_with_registry, repo_sync_for_registered_repository,
};
#[cfg(test)]
use crate::gateway::studio::repo_index::types::RepoIndexSnapshot;
use crate::gateway::studio::repo_index::types::{
    RepoCodeDocument, RepoIndexEntryStatus, RepoIndexPhase, RepoIndexStatusResponse,
};
use crate::search_plane::SearchPlaneService;

use super::collect::{await_analysis_completion, collect_code_documents};
use super::filters::{aggregate_status_response, filter_status_response};
use super::fingerprint::{fingerprint, fingerprint_id, timestamp_now};
use super::task::{
    AdaptiveConcurrencyController, REPO_INDEX_ANALYSIS_TIMEOUT, RepoIndexTask,
    RepoIndexTaskPriority, RepoTaskFeedback, RepoTaskOutcome, repo_index_sync_concurrency_limit,
    should_retry_sync_failure,
};

pub(crate) struct RepoIndexCoordinator {
    project_root: PathBuf,
    plugin_registry: Arc<PluginRegistry>,
    search_plane: SearchPlaneService,
    statuses: Arc<RwLock<BTreeMap<String, RepoIndexEntryStatus>>>,
    fingerprints: Arc<RwLock<HashMap<String, String>>>,
    queued_or_active: Arc<RwLock<HashSet<String>>>,
    active_repo_ids: Arc<RwLock<Vec<String>>>,
    status_snapshot: Arc<Mutex<RepoIndexStatusResponse>>,
    pending: Arc<Mutex<VecDeque<RepoIndexTask>>>,
    notify: Arc<Notify>,
    concurrency: Arc<Mutex<AdaptiveConcurrencyController>>,
    sync_concurrency_limit: usize,
    sync_permits: Arc<Semaphore>,
    started: AtomicBool,
    shutdown_requested: AtomicBool,
    run_task: Mutex<Option<JoinHandle<()>>>,
}

impl RepoIndexCoordinator {
    #[must_use]
    pub(crate) fn new(
        project_root: PathBuf,
        plugin_registry: Arc<PluginRegistry>,
        search_plane: SearchPlaneService,
    ) -> Self {
        let sync_concurrency_limit = repo_index_sync_concurrency_limit();
        Self {
            project_root,
            plugin_registry,
            search_plane,
            statuses: Arc::new(RwLock::new(BTreeMap::new())),
            fingerprints: Arc::new(RwLock::new(HashMap::new())),
            queued_or_active: Arc::new(RwLock::new(HashSet::new())),
            active_repo_ids: Arc::new(RwLock::new(Vec::new())),
            status_snapshot: Arc::new(Mutex::new(RepoIndexStatusResponse::default())),
            pending: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            concurrency: Arc::new(Mutex::new(AdaptiveConcurrencyController::new())),
            sync_concurrency_limit,
            sync_permits: Arc::new(Semaphore::new(sync_concurrency_limit)),
            started: AtomicBool::new(false),
            shutdown_requested: AtomicBool::new(false),
            run_task: Mutex::new(None),
        }
    }

    pub(crate) fn start(self: &Arc<Self>) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        self.shutdown_requested.store(false, Ordering::SeqCst);
        if let Ok(handle) = Handle::try_current() {
            let coordinator = Arc::clone(self);
            let run_task = handle.spawn(async move {
                coordinator.run().await;
            });
            *self
                .run_task
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(run_task);
        }
    }

    pub(crate) fn stop(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        if let Some(run_task) = self
            .run_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            run_task.abort();
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
            let repo_fingerprint = fingerprint(&repository);
            let existing = self
                .fingerprints
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&repository.id)
                .cloned();
            if existing.as_deref() != Some(repo_fingerprint.as_str())
                && self.enqueue_repository(
                    repository,
                    false,
                    true,
                    repo_fingerprint.clone(),
                    RepoIndexTaskPriority::Background,
                )
            {
                enqueued.push(fingerprint_id(&repo_fingerprint));
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
            let repo_fingerprint = fingerprint(&repository);
            if self.enqueue_repository(
                repository,
                refresh,
                refresh,
                repo_fingerprint.clone(),
                RepoIndexTaskPriority::Interactive,
            ) {
                enqueued.push(fingerprint_id(&repo_fingerprint));
            }
        }
        enqueued
    }

    pub(crate) fn status_response(&self, repo_id: Option<&str>) -> RepoIndexStatusResponse {
        let snapshot = self
            .status_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        filter_status_response(snapshot, repo_id)
    }

    pub(crate) async fn acquire_sync_permit(
        &self,
        repo_id: &str,
    ) -> Result<OwnedSemaphorePermit, RepoIntelligenceError> {
        Arc::clone(&self.sync_permits)
            .acquire_owned()
            .await
            .map_err(|_| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id}` sync semaphore was closed while waiting to start remote sync"
                ),
            })
    }

    fn prune_removed(&self, active_ids: &BTreeSet<String>) {
        let removed_repo_ids = self
            .statuses
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .filter(|repo_id| !active_ids.contains(*repo_id))
            .cloned()
            .collect::<Vec<_>>();
        self.statuses
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
        self.active_repo_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|repo_id| active_ids.contains(repo_id));
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|task| active_ids.contains(&task.repository.id));
        for repo_id in removed_repo_ids {
            self.search_plane.clear_repo_publications(repo_id.as_str());
        }
        self.refresh_status_snapshot();
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn enqueue_repository(
        &self,
        repository: RegisteredRepository,
        refresh: bool,
        force: bool,
        repo_fingerprint: String,
        priority: RepoIndexTaskPriority,
    ) -> bool {
        self.enqueue_task(
            RepoIndexTask {
                repository,
                refresh,
                fingerprint: repo_fingerprint,
                priority,
                retry_count: 0,
            },
            force,
        )
    }

    #[allow(clippy::too_many_lines)]
    fn enqueue_task(&self, task: RepoIndexTask, force: bool) -> bool {
        let repo_id = task.repository.id.clone();
        let incoming_priority = task.priority;
        let incoming_refresh = task.refresh;
        let incoming_retry_count = task.retry_count;
        let incoming_repository = task.repository;
        let incoming_fingerprint = task.fingerprint;
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
        let is_same_fingerprint =
            existing_fingerprint.as_deref() == Some(incoming_fingerprint.as_str());
        let already_queued_or_active = self
            .queued_or_active
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&repo_id);

        if already_queued_or_active {
            let mut updated_existing_task = false;
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(index) = pending
                .iter()
                .position(|task| task.repository.id == repo_id)
                && let Some(mut task) = pending.remove(index)
            {
                let fingerprint_changed = task.fingerprint != incoming_fingerprint;
                updated_existing_task = fingerprint_changed
                    || incoming_refresh
                    || matches!(incoming_priority, RepoIndexTaskPriority::Interactive)
                        && !matches!(task.priority, RepoIndexTaskPriority::Interactive);
                if fingerprint_changed {
                    self.fingerprints
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(repo_id.clone(), incoming_fingerprint.clone());
                }
                task.priority = match (task.priority, incoming_priority) {
                    (RepoIndexTaskPriority::Interactive, _)
                    | (_, RepoIndexTaskPriority::Interactive) => RepoIndexTaskPriority::Interactive,
                    _ => RepoIndexTaskPriority::Background,
                };
                task.refresh |= incoming_refresh;
                task.repository = incoming_repository;
                task.fingerprint = incoming_fingerprint;
                task.retry_count = if fingerprint_changed {
                    0
                } else {
                    incoming_retry_count
                };
                match task.priority {
                    RepoIndexTaskPriority::Interactive => pending.push_front(task),
                    RepoIndexTaskPriority::Background => pending.push_back(task),
                }
            }
            drop(pending);
            if updated_existing_task {
                self.refresh_status_snapshot();
                self.notify.notify_one();
            }
            return updated_existing_task;
        }

        if !force
            && is_same_fingerprint
            && let Some(ref status) = existing_status
            && matches!(
                status.phase,
                RepoIndexPhase::Ready | RepoIndexPhase::Unsupported | RepoIndexPhase::Failed
            )
        {
            return false;
        }

        self.fingerprints
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(repo_id.clone(), incoming_fingerprint.clone());
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
                    queue_position: None,
                    last_error: None,
                    last_revision: existing_status
                        .as_ref()
                        .and_then(|status| status.last_revision.clone()),
                    updated_at: Some(timestamp_now()),
                    attempt_count: 0,
                },
            );
        self.refresh_status_snapshot();
        let task = RepoIndexTask {
            repository: incoming_repository,
            refresh: incoming_refresh,
            fingerprint: incoming_fingerprint,
            priority: incoming_priority,
            retry_count: incoming_retry_count,
        };
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match task.priority {
            RepoIndexTaskPriority::Interactive => pending.push_front(task),
            RepoIndexTaskPriority::Background => pending.push_back(task),
        }
        drop(pending);
        self.notify.notify_one();
        true
    }

    async fn run(self: Arc<Self>) {
        let mut running = JoinSet::new();
        loop {
            if self.shutdown_requested.load(Ordering::SeqCst) {
                break;
            }

            self.dispatch_pending_tasks(&mut running);

            if self.shutdown_requested.load(Ordering::SeqCst) {
                break;
            }

            if running.is_empty() {
                self.notify.notified().await;
                continue;
            }

            tokio::select! {
                biased;
                Some(result) = running.join_next() => {
                    self.handle_task_result(result);
                }
                () = self.notify.notified() => {}
            }
        }

        running.abort_all();
        while running.join_next().await.is_some() {}
    }

    fn dispatch_pending_tasks(self: &Arc<Self>, running: &mut JoinSet<RepoTaskFeedback>) {
        loop {
            let target = self.target_parallelism(running.len());
            if running.len() >= target {
                break;
            }

            let Some(task) = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pop_front()
            else {
                break;
            };

            self.mark_active(task.repository.id.as_str());
            let coordinator = Arc::clone(self);
            running.spawn(async move { coordinator.process_task(task).await });
        }
    }

    fn target_parallelism(&self, active_count: usize) -> usize {
        let queued = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        let mut controller = self
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        controller.target_limit(queued, active_count)
    }

    fn handle_task_result(&self, result: Result<RepoTaskFeedback, tokio::task::JoinError>) {
        let feedback = match result {
            Ok(feedback) => feedback,
            Err(error) => {
                let mut controller = self
                    .concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                controller.record_failure();
                self.refresh_status_snapshot();
                if error.is_panic() {
                    self.notify.notify_one();
                }
                return;
            }
        };

        match feedback.outcome {
            RepoTaskOutcome::Success { revision } => {
                self.record_repo_status(
                    feedback.repo_id.as_str(),
                    RepoIndexPhase::Ready,
                    revision,
                    None,
                );
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_success(
                        feedback.elapsed,
                        self.pending
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .len(),
                    );
            }
            RepoTaskOutcome::Failure { revision, error } => {
                self.record_failure_status(feedback.repo_id.as_str(), &error, revision);
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_failure();
            }
            RepoTaskOutcome::Requeued { task, error } => {
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_failure();
                self.release_repo(feedback.repo_id.as_str());
                if !self.enqueue_task(task, true) {
                    self.record_failure_status(feedback.repo_id.as_str(), &error, None);
                }
                self.notify.notify_one();
                return;
            }
            RepoTaskOutcome::Skipped => {}
        }
        self.release_repo(feedback.repo_id.as_str());
        self.notify.notify_one();
    }

    #[allow(clippy::too_many_lines)]
    async fn process_task(self: Arc<Self>, task: RepoIndexTask) -> RepoTaskFeedback {
        let repo_id = task.repository.id.clone();
        let started_at = Instant::now();
        if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Skipped,
            };
        }

        self.bump_status(&repo_id, RepoIndexPhase::Checking, None, None);

        let sync_result = self
            .run_repository_sync(repo_id.as_str(), task.repository.clone(), task.refresh)
            .await;
        let sync_result = match sync_result {
            Ok(result) => result,
            Err(error) if should_retry_sync_failure(&error, task.retry_count) => {
                let mut retry_task = task.clone();
                retry_task.retry_count = retry_task.retry_count.saturating_add(1);
                return RepoTaskFeedback {
                    repo_id,
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Requeued {
                        task: retry_task,
                        error,
                    },
                };
            }
            Err(error) => {
                return RepoTaskFeedback {
                    repo_id,
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Failure {
                        revision: None,
                        error,
                    },
                };
            }
        };

        if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Skipped,
            };
        }

        self.bump_status(
            &repo_id,
            RepoIndexPhase::Indexing,
            sync_result.revision.clone(),
            None,
        );

        let analysis = self.run_repository_analysis(task.repository.clone()).await;
        match analysis {
            Ok(analysis) => {
                if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
                    return RepoTaskFeedback {
                        repo_id,
                        elapsed: started_at.elapsed(),
                        outcome: RepoTaskOutcome::Skipped,
                    };
                }

                let code_documents = match self
                    .collect_code_documents_for_task(
                        repo_id.as_str(),
                        task.fingerprint.as_str(),
                        sync_result.checkout_path.as_str(),
                    )
                    .await
                {
                    Ok(Some(code_documents)) => code_documents,
                    Ok(None) => {
                        return RepoTaskFeedback {
                            repo_id,
                            elapsed: started_at.elapsed(),
                            outcome: RepoTaskOutcome::Skipped,
                        };
                    }
                    Err(error) => {
                        return RepoTaskFeedback {
                            repo_id,
                            elapsed: started_at.elapsed(),
                            outcome: RepoTaskOutcome::Failure {
                                revision: sync_result.revision,
                                error,
                            },
                        };
                    }
                };

                if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
                    return RepoTaskFeedback {
                        repo_id,
                        elapsed: started_at.elapsed(),
                        outcome: RepoTaskOutcome::Skipped,
                    };
                }

                if let Err(error) = self
                    .search_plane
                    .publish_repo_entities_with_revision(
                        repo_id.as_str(),
                        &analysis,
                        &code_documents,
                        sync_result.revision.as_deref(),
                    )
                    .await
                {
                    let failed_repo_id = repo_id.clone();
                    return RepoTaskFeedback {
                        repo_id,
                        elapsed: started_at.elapsed(),
                        outcome: RepoTaskOutcome::Failure {
                            revision: sync_result.revision,
                            error: RepoIntelligenceError::AnalysisFailed {
                                message: format!(
                                    "repo `{failed_repo_id}` repo-entity publish failed: {error}"
                                ),
                            },
                        },
                    };
                }

                if let Err(error) = self
                    .search_plane
                    .publish_repo_content_chunks_with_revision(
                        repo_id.as_str(),
                        &code_documents,
                        sync_result.revision.as_deref(),
                    )
                    .await
                {
                    let failed_repo_id = repo_id.clone();
                    return RepoTaskFeedback {
                        repo_id,
                        elapsed: started_at.elapsed(),
                        outcome: RepoTaskOutcome::Failure {
                            revision: sync_result.revision,
                            error: RepoIntelligenceError::AnalysisFailed {
                                message: format!(
                                    "repo `{failed_repo_id}` repo-content chunk publish failed: {error}"
                                ),
                            },
                        },
                    };
                }

                RepoTaskFeedback {
                    repo_id: repo_id.clone(),
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Success {
                        revision: sync_result.revision,
                    },
                }
            }
            Err(error) => RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Failure {
                    revision: sync_result.revision,
                    error,
                },
            },
        }
    }

    fn bump_status(
        &self,
        repo_id: &str,
        phase: RepoIndexPhase,
        last_revision: Option<String>,
        last_error: Option<String>,
    ) {
        self.record_repo_status(repo_id, phase, last_revision, last_error);
    }

    fn next_attempt_count(&self, repo_id: &str) -> usize {
        self.statuses
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(repo_id)
            .map_or(1, |status| status.attempt_count.saturating_add(1))
    }

    #[cfg(test)]
    pub(crate) fn set_snapshot_for_test(&self, _snapshot: &Arc<RepoIndexSnapshot>) {
        let _ = &self.status_snapshot;
    }

    #[cfg(test)]
    pub(crate) fn set_status_for_test(&self, status: RepoIndexEntryStatus) {
        self.statuses
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(status.repo_id.clone(), status);
        self.refresh_status_snapshot();
    }

    #[cfg(test)]
    pub(super) fn set_concurrency_for_test(&self, controller: AdaptiveConcurrencyController) {
        *self
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = controller;
        self.refresh_status_snapshot();
    }

    #[cfg(test)]
    pub(super) fn mark_active_for_test(&self, repo_id: &str) {
        self.mark_active(repo_id);
    }

    #[cfg(test)]
    pub(super) fn pending_repo_ids_for_test(&self) -> Vec<String> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .map(|task| task.repository.id.clone())
            .collect()
    }

    pub(super) fn record_repo_status(
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
                    queue_position: None,
                    last_error,
                    last_revision,
                    updated_at: Some(timestamp_now()),
                    attempt_count,
                },
            );
        self.refresh_status_snapshot();
    }

    async fn run_repository_analysis(
        &self,
        repository: RegisteredRepository,
    ) -> Result<crate::analyzers::RepositoryAnalysisOutput, RepoIntelligenceError> {
        let repo_id = repository.id.clone();
        let project_root = self.project_root.clone();
        let plugin_registry = Arc::clone(&self.plugin_registry);
        let task = tokio::task::spawn_blocking(move || {
            analyze_registered_repository_with_registry(
                &repository,
                project_root.as_path(),
                plugin_registry.as_ref(),
            )
        });
        await_analysis_completion(repo_id.as_str(), task, REPO_INDEX_ANALYSIS_TIMEOUT).await
    }

    async fn run_repository_sync(
        &self,
        repo_id: &str,
        repository: RegisteredRepository,
        refresh: bool,
    ) -> Result<RepoSyncResult, RepoIntelligenceError> {
        let repo_id = repo_id.to_string();
        let repo_id_for_worker = repo_id.clone();
        let project_root = self.project_root.clone();
        let mode = if refresh {
            RepoSyncMode::Refresh
        } else {
            RepoSyncMode::Ensure
        };
        let permit = self.acquire_sync_permit(repo_id.as_str()).await?;
        self.bump_status(repo_id.as_str(), RepoIndexPhase::Syncing, None, None);
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            repo_sync_for_registered_repository(
                &RepoSyncQuery {
                    repo_id: repo_id_for_worker,
                    mode,
                },
                &repository,
                project_root.as_path(),
            )
        });
        match task.await {
            Ok(result) => result,
            Err(error) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo sync worker for `{repo_id}` terminated unexpectedly: {error}"
                ),
            }),
        }
    }

    async fn collect_code_documents_for_task(
        &self,
        repo_id: &str,
        fingerprint: &str,
        checkout_path: &str,
    ) -> Result<Option<Vec<RepoCodeDocument>>, RepoIntelligenceError> {
        let repo_id = repo_id.to_string();
        let fingerprint = fingerprint.to_string();
        let checkout_path = checkout_path.to_string();
        let fingerprints = Arc::clone(&self.fingerprints);
        let repo_id_for_error = repo_id.clone();
        let repo_id_for_worker = repo_id.clone();
        let task = tokio::task::spawn_blocking(move || {
            Ok::<Option<Vec<RepoCodeDocument>>, RepoIntelligenceError>(collect_code_documents(
                Path::new(checkout_path.as_str()),
                || {
                    let current = fingerprints
                        .read()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get(&repo_id_for_worker)
                        .cloned();
                    current.as_deref() != Some(fingerprint.as_str())
                },
            ))
        });

        match tokio::time::timeout(REPO_INDEX_ANALYSIS_TIMEOUT, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id_for_error}` code document worker terminated unexpectedly: {error}"
                ),
            }),
            Err(_) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id_for_error}` code document collection timed out after {}s while indexing was running",
                    REPO_INDEX_ANALYSIS_TIMEOUT.as_secs()
                ),
            }),
        }
    }

    fn fingerprint_matches(&self, repo_id: &str, fingerprint: &str) -> bool {
        self.fingerprints
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(repo_id)
            .is_some_and(|current| current == fingerprint)
    }

    fn mark_active(&self, repo_id: &str) {
        let mut active_repo_ids = self
            .active_repo_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active_repo_ids.iter().any(|active| active == repo_id) {
            return;
        }
        active_repo_ids.push(repo_id.to_string());
        drop(active_repo_ids);
        self.refresh_status_snapshot();
    }

    fn release_repo(&self, repo_id: &str) {
        self.active_repo_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|active| active != repo_id);
        self.queued_or_active
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(repo_id);
        self.refresh_status_snapshot();
    }

    fn record_failure_status(
        &self,
        repo_id: &str,
        error: &RepoIntelligenceError,
        last_revision: Option<String>,
    ) {
        let phase = if matches!(
            error,
            RepoIntelligenceError::UnsupportedRepositoryLayout { .. }
        ) {
            RepoIndexPhase::Unsupported
        } else {
            RepoIndexPhase::Failed
        };
        self.record_repo_status(repo_id, phase, last_revision, Some(error.to_string()));
    }

    fn refresh_status_snapshot(&self) {
        let queue_positions = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .enumerate()
            .map(|(index, task)| (task.repository.id.clone(), index.saturating_add(1)))
            .collect::<HashMap<_, _>>();
        let repos = self
            .statuses
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .cloned()
            .map(|mut status| {
                status.queue_position = if matches!(status.phase, RepoIndexPhase::Queued) {
                    queue_positions.get(&status.repo_id).copied()
                } else {
                    None
                };
                status
            })
            .collect::<Vec<_>>();
        let active_repo_ids = self
            .active_repo_ids
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let concurrency = self
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot();
        let snapshot = aggregate_status_response(
            repos,
            active_repo_ids,
            concurrency,
            self.sync_concurrency_limit,
        );
        self.search_plane.synchronize_repo_runtime(&snapshot);
        *self
            .status_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = snapshot;
    }
}
