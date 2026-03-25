use chrono::Utc;
use xiuxian_vector::{TableInfo, VectorStoreError};

use super::helpers::{
    REPO_MAINTENANCE_SHUTDOWN_MESSAGE, repo_active_epoch, repo_runtime_status_for_record,
};
use crate::search_plane::service::core::types::{
    RepoCompactionTask, RepoMaintenanceTask, RepoPrewarmTask, SearchPlaneService,
};
use crate::search_plane::service::helpers::repo_corpus_staging_epoch;
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenanceStatus, SearchRepoCorpusRecord, SearchRepoPublicationRecord,
};

impl SearchPlaneService {
    pub(crate) fn stop_repo_maintenance(&self) {
        let (worker_handle, waiters) = {
            let mut runtime = self
                .repo_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            runtime.shutdown_requested = true;
            runtime.worker_running = false;
            runtime.active_task = None;
            runtime.queue.clear();
            runtime.in_flight.clear();
            let worker_handle = runtime.worker_handle.take();
            let waiters = std::mem::take(&mut runtime.waiters);
            (worker_handle, waiters)
        };
        if let Some(worker_handle) = worker_handle {
            worker_handle.abort();
        }
        for waiters in waiters.into_values() {
            for waiter in waiters {
                let _ = waiter.send(Err(REPO_MAINTENANCE_SHUTDOWN_MESSAGE.to_string()));
            }
        }
    }

    pub(crate) async fn prewarm_repo_table(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        table_name: &str,
        projected_columns: &[&str],
    ) -> Result<(), VectorStoreError> {
        if self.repo_maintenance_shutdown_requested() {
            return Err(VectorStoreError::General(
                REPO_MAINTENANCE_SHUTDOWN_MESSAGE.to_string(),
            ));
        }
        let task = RepoMaintenanceTask::Prewarm(RepoPrewarmTask {
            corpus,
            repo_id: repo_id.to_string(),
            table_name: table_name.to_string(),
            projected_columns: projected_columns
                .iter()
                .map(|column| (*column).to_string())
                .collect(),
        });
        let task_key = task.task_key();
        let (receiver, enqueued, start_worker) =
            self.register_repo_maintenance_task(task.clone(), true);
        if !enqueued
            && !self.repo_maintenance_shutdown_requested()
            && !self.repo_maintenance_task_is_live(&task_key)
        {
            self.complete_repo_maintenance_task(
                &task_key,
                Err("stale repo maintenance claim without queued or active worker".to_string()),
            );
            return self.run_repo_maintenance_task(task).await;
        }
        self.ensure_repo_maintenance_worker(start_worker).await;
        self.await_repo_maintenance(receiver, &task_key).await
    }

    pub(crate) async fn run_repo_maintenance_task(
        &self,
        task: RepoMaintenanceTask,
    ) -> Result<(), VectorStoreError> {
        match task {
            RepoMaintenanceTask::Prewarm(task) => {
                let projected_columns = task
                    .projected_columns
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                self.prewarm_named_table(task.corpus, task.table_name.as_str(), &projected_columns)
                    .await?;
                self.record_repo_corpus_prewarm(task.corpus, task.repo_id.as_str())
                    .await;
                Ok(())
            }
            RepoMaintenanceTask::Compaction(task) => {
                self.run_repo_compaction_task(task).await;
                Ok(())
            }
        }
    }

    pub(crate) fn next_repo_publication_maintenance(
        &self,
        previous_record: Option<&SearchRepoCorpusRecord>,
        next_row_count: u64,
    ) -> SearchMaintenanceStatus {
        let mut maintenance = previous_record
            .and_then(|record| record.maintenance.clone())
            .unwrap_or_default();
        let publish_count = maintenance.publish_count_since_compaction.saturating_add(1);
        maintenance.publish_count_since_compaction = publish_count;
        maintenance.compaction_running = false;
        maintenance.compaction_pending = self.coordinator.maintenance_policy().should_compact(
            publish_count,
            maintenance.last_compacted_row_count,
            next_row_count,
        );
        maintenance
    }

    pub(crate) async fn schedule_repo_compaction_if_needed(&self, record: &SearchRepoCorpusRecord) {
        let Some(compaction_task) = self.repo_compaction_task(record) else {
            return;
        };
        let task = RepoMaintenanceTask::Compaction(compaction_task);
        let (_receiver, enqueued, start_worker) = self.register_repo_maintenance_task(task, false);
        if !enqueued {
            return;
        }
        self.ensure_repo_maintenance_worker(start_worker).await;
    }

    fn repo_compaction_task(&self, record: &SearchRepoCorpusRecord) -> Option<RepoCompactionTask> {
        let publication = record.publication.as_ref()?;
        let maintenance = record.maintenance.as_ref()?;
        if !maintenance.compaction_pending {
            return None;
        }
        let reason = self.coordinator.maintenance_policy().compaction_reason(
            maintenance.publish_count_since_compaction,
            maintenance.last_compacted_row_count,
            publication.row_count,
        )?;
        Some(RepoCompactionTask {
            corpus: record.corpus,
            repo_id: record.repo_id.clone(),
            publication_id: publication.publication_id.clone(),
            table_name: publication.table_name.clone(),
            row_count: publication.row_count,
            reason,
        })
    }

    async fn run_repo_compaction_task(&self, task: RepoCompactionTask) {
        let store = match self.open_store(task.corpus).await {
            Ok(store) => store,
            Err(error) => {
                log::warn!(
                    "search-plane repo compaction failed to open store for {} repo {} table {}: {}",
                    task.corpus,
                    task.repo_id,
                    task.table_name,
                    error
                );
                let _ = self.stop_repo_compaction(&task, true).await;
                return;
            }
        };
        match store.compact(task.table_name.as_str()).await {
            Ok(_) => match store.get_table_info(task.table_name.as_str()).await {
                Ok(table_info) => {
                    let _ = self.complete_repo_compaction(&task, &table_info).await;
                }
                Err(error) => {
                    log::warn!(
                        "search-plane repo compaction failed to inspect {} repo {} table {} after compact: {}",
                        task.corpus,
                        task.repo_id,
                        task.table_name,
                        error
                    );
                    let _ = self.stop_repo_compaction(&task, true).await;
                }
            },
            Err(error) => {
                log::warn!(
                    "search-plane repo compaction failed for {} repo {} table {}: {}",
                    task.corpus,
                    task.repo_id,
                    task.table_name,
                    error
                );
                let _ = self.stop_repo_compaction(&task, true).await;
            }
        }
    }

    pub(crate) async fn mark_repo_compaction_running(&self, task: &RepoCompactionTask) -> bool {
        self.update_repo_compaction_record(task, |_publication, maintenance| {
            maintenance.compaction_running = true;
        })
        .await
    }

    pub(crate) async fn stop_repo_compaction(
        &self,
        task: &RepoCompactionTask,
        keep_pending: bool,
    ) -> bool {
        self.update_repo_compaction_record(task, |_publication, maintenance| {
            maintenance.compaction_running = false;
            maintenance.compaction_pending = keep_pending;
        })
        .await
    }

    async fn complete_repo_compaction(
        &self,
        task: &RepoCompactionTask,
        table_info: &TableInfo,
    ) -> bool {
        self.update_repo_compaction_record(task, |publication, maintenance| {
            publication.fragment_count =
                u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX);
            maintenance.compaction_running = false;
            maintenance.compaction_pending = false;
            maintenance.publish_count_since_compaction = 0;
            maintenance.last_compacted_at = Some(Utc::now().to_rfc3339());
            maintenance.last_compaction_reason = Some(task.reason.as_str().to_string());
            maintenance.last_compacted_row_count = Some(table_info.num_rows);
        })
        .await
    }

    async fn update_repo_compaction_record<F>(&self, task: &RepoCompactionTask, mutate: F) -> bool
    where
        F: FnOnce(&mut SearchRepoPublicationRecord, &mut SearchMaintenanceStatus),
    {
        let key = (task.corpus, task.repo_id.clone());
        let mut record = match self
            .repo_corpus_record_for_reads(task.corpus, task.repo_id.as_str())
            .await
        {
            Some(record) => record,
            None => return false,
        };
        let Some(publication) = record.publication.as_mut() else {
            return false;
        };
        if publication.publication_id != task.publication_id {
            return false;
        }
        let mut maintenance = record.maintenance.clone().unwrap_or_default();
        mutate(publication, &mut maintenance);
        record.maintenance = Some(maintenance);
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, record.clone());
        self.cache.set_repo_corpus_record(&record).await;
        self.cache
            .set_repo_corpus_snapshot(&self.current_repo_corpus_snapshot_record())
            .await;
        self.synchronize_repo_corpus_statuses_from_runtime().await;
        true
    }

    async fn record_repo_corpus_prewarm(&self, corpus: SearchCorpusKind, repo_id: &str) {
        let mut record = self
            .repo_corpus_record_for_reads(corpus, repo_id)
            .await
            .unwrap_or_else(|| {
                SearchRepoCorpusRecord::new(
                    corpus,
                    repo_id.to_string(),
                    self.repo_runtime_state(repo_id)
                        .map(|state| Self::runtime_record_from_state(repo_id, &state)),
                    None,
                )
            });
        let mut repo_records = self.repo_corpus_snapshot_for_reads().await;
        repo_records.insert((corpus, repo_id.to_string()), record.clone());
        let relevant_records = repo_records
            .values()
            .filter(|candidate| candidate.corpus == corpus)
            .cloned()
            .collect::<Vec<_>>();
        let active_epoch = repo_active_epoch(corpus, relevant_records.as_slice());
        let runtime_statuses = relevant_records
            .iter()
            .filter_map(repo_runtime_status_for_record)
            .collect::<Vec<_>>();
        let prewarmed_epoch =
            repo_corpus_staging_epoch(corpus, &runtime_statuses, active_epoch).or(active_epoch);
        let mut maintenance = record.maintenance.unwrap_or_default();
        maintenance.prewarm_running = false;
        maintenance.last_prewarmed_at = Some(Utc::now().to_rfc3339());
        maintenance.last_prewarmed_epoch = prewarmed_epoch;
        record.maintenance = Some(maintenance);
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((corpus, repo_id.to_string()), record.clone());
        self.cache.set_repo_corpus_record(&record).await;
        self.cache
            .set_repo_corpus_snapshot(&self.current_repo_corpus_snapshot_record())
            .await;
        self.synchronize_repo_corpus_statuses_from_runtime().await;
    }

    pub(crate) async fn mark_repo_prewarm_running(&self, corpus: SearchCorpusKind, repo_id: &str) {
        self.update_repo_prewarm_record(corpus, repo_id, |maintenance| {
            maintenance.prewarm_running = true;
        })
        .await;
    }

    pub(crate) async fn stop_repo_prewarm(&self, corpus: SearchCorpusKind, repo_id: &str) {
        self.update_repo_prewarm_record(corpus, repo_id, |maintenance| {
            maintenance.prewarm_running = false;
        })
        .await;
    }

    async fn update_repo_prewarm_record<F>(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        mutate: F,
    ) where
        F: FnOnce(&mut SearchMaintenanceStatus),
    {
        let mut record = self
            .repo_corpus_record_for_reads(corpus, repo_id)
            .await
            .unwrap_or_else(|| {
                SearchRepoCorpusRecord::new(
                    corpus,
                    repo_id.to_string(),
                    self.repo_runtime_state(repo_id)
                        .map(|state| Self::runtime_record_from_state(repo_id, &state)),
                    None,
                )
            });
        let mut maintenance = record.maintenance.unwrap_or_default();
        mutate(&mut maintenance);
        record.maintenance = Some(maintenance);
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((corpus, repo_id.to_string()), record.clone());
        self.cache.set_repo_corpus_record(&record).await;
        self.cache
            .set_repo_corpus_snapshot(&self.current_repo_corpus_snapshot_record())
            .await;
        self.synchronize_repo_corpus_statuses_from_runtime().await;
    }

    pub(crate) fn clear_repo_maintenance_for_repo(&self, repo_id: &str) {
        let mut runtime = self
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cleared_keys = runtime
            .in_flight
            .iter()
            .filter(|(_, candidate_repo_id, _, _)| candidate_repo_id == repo_id)
            .cloned()
            .collect::<Vec<_>>();
        runtime
            .in_flight
            .retain(|(_, candidate_repo_id, _, _)| candidate_repo_id != repo_id);
        runtime
            .queue
            .retain(|queued| queued.task.repo_id() != repo_id);
        let drained_waiter_keys = runtime
            .waiters
            .keys()
            .filter(|(_, candidate_repo_id, _, _)| candidate_repo_id == repo_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut drained_waiters = drained_waiter_keys
            .into_iter()
            .filter_map(|task_key| {
                runtime
                    .waiters
                    .remove(&task_key)
                    .map(|waiters| (task_key, waiters))
            })
            .collect::<Vec<_>>();
        drop(runtime);
        for task_key in cleared_keys {
            if let Some((_, waiters)) = drained_waiters
                .iter_mut()
                .find(|(candidate_task_key, _)| candidate_task_key == &task_key)
            {
                for waiter in waiters.drain(..) {
                    let _ = waiter.send(Err(format!(
                        "repo maintenance task for {repo_id} was cleared before completion"
                    )));
                }
            }
        }
    }
}
