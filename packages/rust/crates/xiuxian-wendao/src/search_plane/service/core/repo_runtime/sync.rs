use crate::gateway::studio::repo_index::RepoIndexStatusResponse;
use crate::search_plane::SearchCorpusKind;
use crate::search_plane::service::core::types::SearchPlaneService;

impl SearchPlaneService {
    pub(crate) fn synchronize_repo_runtime(&self, repo_status: &RepoIndexStatusResponse) {
        let runtime_records = Self::repo_runtime_records(repo_status);
        let next_runtime = Self::next_repo_runtime_states(repo_status);
        let (updated_records, removed_repo_ids) =
            self.repo_runtime_delta(runtime_records.as_slice(), &next_runtime);
        self.apply_repo_runtime_to_memory(runtime_records.as_slice(), removed_repo_ids.as_slice());
        if updated_records.is_empty() && removed_repo_ids.is_empty() {
            return;
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let service = self.clone();
            handle.spawn(async move {
                service
                    .refresh_repo_runtime_cache(updated_records, removed_repo_ids, runtime_records)
                    .await;
            });
        }
    }

    async fn refresh_repo_runtime_cache(
        &self,
        updated_records: Vec<crate::search_plane::SearchRepoRuntimeRecord>,
        removed_repo_ids: Vec<String>,
        runtime_records: Vec<crate::search_plane::SearchRepoRuntimeRecord>,
    ) {
        let _ = updated_records;
        self.delete_removed_repo_runtime_records(removed_repo_ids.as_slice())
            .await;
        self.refresh_repo_corpus_records(runtime_records.as_slice())
            .await;
        self.persist_repo_corpus_snapshot().await;
        self.synchronize_repo_corpus_statuses_from_runtime().await;
    }

    async fn delete_removed_repo_runtime_records(&self, removed_repo_ids: &[String]) {
        for repo_id in removed_repo_ids {
            for corpus in [
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ] {
                self.repo_corpus_records
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&(corpus, repo_id.clone()));
                self.cache
                    .delete_repo_corpus_record(corpus, repo_id.as_str())
                    .await;
            }
        }
    }

    async fn refresh_repo_corpus_records(
        &self,
        runtime_records: &[crate::search_plane::SearchRepoRuntimeRecord],
    ) {
        for runtime in runtime_records {
            for corpus in [
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ] {
                let existing_record = self
                    .repo_corpus_record_for_reads(corpus, runtime.repo_id.as_str())
                    .await;
                let publication = existing_record
                    .as_ref()
                    .and_then(|record| record.publication.clone())
                    .or_else(|| self.cached_repo_publication(corpus, runtime.repo_id.as_str()));
                let maintenance = existing_record
                    .as_ref()
                    .and_then(|record| record.maintenance.clone());
                let record = crate::search_plane::SearchRepoCorpusRecord::new(
                    corpus,
                    runtime.repo_id.clone(),
                    Some(runtime.clone()),
                    publication,
                )
                .with_maintenance(maintenance);
                self.repo_corpus_records
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert((corpus, runtime.repo_id.clone()), record.clone());
                self.cache.set_repo_corpus_record(&record).await;
            }
        }
    }

    async fn persist_repo_corpus_snapshot(&self) {
        let corpus_snapshot = self.current_repo_corpus_snapshot_record();
        if corpus_snapshot.records.is_empty() {
            self.cache.delete_repo_corpus_snapshot().await;
        } else {
            self.cache.set_repo_corpus_snapshot(&corpus_snapshot).await;
        }
    }
}
