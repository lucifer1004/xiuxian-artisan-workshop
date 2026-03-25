use std::collections::BTreeMap;

use super::types::{RepoRuntimeState, SearchPlaneService};
use crate::gateway::studio::repo_index::{RepoIndexPhase, RepoIndexStatusResponse};
use crate::search_plane::{
    SearchCorpusKind, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
    SearchRepoRuntimeRecord,
};

impl SearchPlaneService {
    fn repo_search_publication_state_from_records(
        entity_record: Option<&SearchRepoCorpusRecord>,
        content_record: Option<&SearchRepoCorpusRecord>,
    ) -> super::types::RepoSearchPublicationState {
        let entity_published = entity_record
            .and_then(|record| record.publication.as_ref())
            .is_some();
        let content_published = content_record
            .and_then(|record| record.publication.as_ref())
            .is_some();
        let runtime = entity_record
            .and_then(|record| record.runtime.as_ref())
            .or_else(|| content_record.and_then(|record| record.runtime.as_ref()))
            .map(RepoRuntimeState::from_record);
        let availability = if entity_published || content_published {
            super::types::RepoSearchAvailability::Searchable
        } else if matches!(
            runtime.as_ref().map(|state| state.phase),
            Some(RepoIndexPhase::Unsupported | RepoIndexPhase::Failed)
        ) {
            super::types::RepoSearchAvailability::Skipped
        } else {
            super::types::RepoSearchAvailability::Pending
        };
        super::types::RepoSearchPublicationState {
            entity_published,
            content_published,
            availability,
        }
    }

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

    fn repo_runtime_records(repo_status: &RepoIndexStatusResponse) -> Vec<SearchRepoRuntimeRecord> {
        repo_status
            .repos
            .iter()
            .map(SearchRepoRuntimeRecord::from_status)
            .collect()
    }

    fn next_repo_runtime_states(
        repo_status: &RepoIndexStatusResponse,
    ) -> BTreeMap<String, RepoRuntimeState> {
        repo_status
            .repos
            .iter()
            .map(|status| {
                (
                    status.repo_id.clone(),
                    RepoRuntimeState::from_status(status),
                )
            })
            .collect()
    }

    fn repo_runtime_delta(
        &self,
        runtime_records: &[SearchRepoRuntimeRecord],
        next_runtime: &BTreeMap<String, RepoRuntimeState>,
    ) -> (Vec<SearchRepoRuntimeRecord>, Vec<String>) {
        let (updated_records, removed_repo_ids) = {
            let current_runtime = self.current_repo_runtime_states();
            let removed_repo_ids = current_runtime
                .keys()
                .filter(|repo_id| !next_runtime.contains_key(*repo_id))
                .cloned()
                .collect::<Vec<_>>();
            let updated_records = runtime_records
                .iter()
                .filter(|status| {
                    current_runtime.get(status.repo_id.as_str())
                        != next_runtime.get(status.repo_id.as_str())
                })
                .cloned()
                .collect::<Vec<_>>();
            (updated_records, removed_repo_ids)
        };
        (updated_records, removed_repo_ids)
    }

    fn apply_repo_runtime_to_memory(
        &self,
        runtime_records: &[SearchRepoRuntimeRecord],
        removed_repo_ids: &[String],
    ) {
        let mut current_records = self
            .repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for repo_id in removed_repo_ids {
            for corpus in [
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ] {
                current_records.remove(&(corpus, repo_id.clone()));
            }
        }
        for runtime in runtime_records {
            Self::upsert_repo_runtime_records(&mut current_records, runtime);
        }
    }

    fn upsert_repo_runtime_records(
        current_records: &mut BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord>,
        runtime: &SearchRepoRuntimeRecord,
    ) {
        for corpus in [
            SearchCorpusKind::RepoEntity,
            SearchCorpusKind::RepoContentChunk,
        ] {
            let key = (corpus, runtime.repo_id.clone());
            match current_records.get_mut(&key) {
                Some(record) => {
                    record.runtime = Some(runtime.clone());
                }
                None => {
                    current_records.insert(
                        key,
                        SearchRepoCorpusRecord::new(
                            corpus,
                            runtime.repo_id.clone(),
                            Some(runtime.clone()),
                            None,
                        ),
                    );
                }
            }
        }
    }

    async fn refresh_repo_runtime_cache(
        &self,
        updated_records: Vec<SearchRepoRuntimeRecord>,
        removed_repo_ids: Vec<String>,
        runtime_records: Vec<SearchRepoRuntimeRecord>,
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

    async fn refresh_repo_corpus_records(&self, runtime_records: &[SearchRepoRuntimeRecord]) {
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
                let record = SearchRepoCorpusRecord::new(
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

    #[cfg(test)]
    pub(crate) async fn repo_search_publication_state(
        &self,
        repo_id: &str,
    ) -> super::types::RepoSearchPublicationState {
        let entity_record = self
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, repo_id)
            .await;
        let content_record = self
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
            .await;
        Self::repo_search_publication_state_from_records(
            entity_record.as_ref(),
            content_record.as_ref(),
        )
    }

    pub(crate) async fn repo_search_publication_states(
        &self,
        repo_ids: &[String],
    ) -> BTreeMap<String, super::types::RepoSearchPublicationState> {
        let records = self.repo_corpus_snapshot_for_reads().await;
        repo_ids
            .iter()
            .map(|repo_id| {
                let entity_record = records.get(&(SearchCorpusKind::RepoEntity, repo_id.clone()));
                let content_record =
                    records.get(&(SearchCorpusKind::RepoContentChunk, repo_id.clone()));
                (
                    repo_id.clone(),
                    Self::repo_search_publication_state_from_records(entity_record, content_record),
                )
            })
            .collect()
    }

    pub(crate) fn repo_runtime_state(&self, repo_id: &str) -> Option<RepoRuntimeState> {
        self.current_repo_runtime_states().remove(repo_id)
    }

    fn current_repo_runtime_states(&self) -> BTreeMap<String, RepoRuntimeState> {
        let mut runtime = BTreeMap::new();
        for record in self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
        {
            if let Some(runtime_record) = record.runtime.as_ref() {
                runtime
                    .entry(record.repo_id.clone())
                    .or_insert_with(|| RepoRuntimeState::from_record(runtime_record));
            }
        }
        runtime
    }

    fn cached_repo_publication(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<crate::search_plane::SearchRepoPublicationRecord> {
        self.repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(corpus, repo_id.to_string()))
            .and_then(|record| record.publication.clone())
    }

    fn cached_repo_corpus_record(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        self.repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(corpus, repo_id.to_string()))
            .cloned()
    }

    pub(crate) fn runtime_record_from_state(
        repo_id: &str,
        state: &RepoRuntimeState,
    ) -> SearchRepoRuntimeRecord {
        SearchRepoRuntimeRecord {
            repo_id: repo_id.to_string(),
            phase: state.phase,
            last_revision: state.last_revision.clone(),
            last_error: state.last_error.clone(),
            updated_at: state.updated_at.clone(),
        }
    }

    fn reconcile_repo_corpus_record(
        &self,
        mut record: SearchRepoCorpusRecord,
    ) -> (SearchRepoCorpusRecord, bool) {
        let mut changed = false;
        if let Some(runtime) = self.repo_runtime_state(record.repo_id.as_str()) {
            let runtime_record = Self::runtime_record_from_state(record.repo_id.as_str(), &runtime);
            if record.runtime.as_ref() != Some(&runtime_record) {
                record.runtime = Some(runtime_record);
                changed = true;
            }
        }
        if let Some(publication) =
            self.cached_repo_publication(record.corpus, record.repo_id.as_str())
            && record.publication.as_ref() != Some(&publication)
        {
            record.publication = Some(publication);
            changed = true;
        }
        (record, changed)
    }

    fn reconcile_repo_corpus_record_for_reads(
        &self,
        record: SearchRepoCorpusRecord,
    ) -> (SearchRepoCorpusRecord, bool) {
        self.reconcile_repo_corpus_record(record)
    }

    pub(crate) fn current_repo_corpus_snapshot_record(&self) -> SearchRepoCorpusSnapshotRecord {
        let records = self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .cloned()
            .collect();
        SearchRepoCorpusSnapshotRecord { records }
    }

    pub(super) async fn repo_corpus_snapshot_for_reads(
        &self,
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        let current = self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if !current.is_empty() {
            let mut changed_records = Vec::new();
            let mut records = BTreeMap::new();
            for (key, record) in current {
                let (record, changed) = self.reconcile_repo_corpus_record_for_reads(record);
                if changed {
                    changed_records.push(record.clone());
                }
                records.insert(key, record);
            }
            if !changed_records.is_empty() {
                *self
                    .repo_corpus_records
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = records.clone();
                for record in &changed_records {
                    self.cache.set_repo_corpus_record(record).await;
                }
                self.cache
                    .set_repo_corpus_snapshot(&SearchRepoCorpusSnapshotRecord {
                        records: records.values().cloned().collect(),
                    })
                    .await;
            }
            return records;
        }
        if let Some(snapshot) = self.cache.get_repo_corpus_snapshot().await {
            let mut records = BTreeMap::new();
            for record in snapshot.records {
                let (record, _) = self.reconcile_repo_corpus_record_for_reads(record);
                records.insert((record.corpus, record.repo_id.clone()), record);
            }
            *self
                .repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = records.clone();
            return records;
        }
        BTreeMap::new()
    }

    pub(crate) async fn repo_corpus_record_for_reads(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        if let Some(record) = self.cached_repo_corpus_record(corpus, repo_id) {
            let (record, changed) = self.reconcile_repo_corpus_record_for_reads(record);
            if changed {
                self.repo_corpus_records
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert((corpus, repo_id.to_string()), record.clone());
                self.cache.set_repo_corpus_record(&record).await;
            }
            return Some(record);
        }
        if let Some(record) = self.cache.get_repo_corpus_record(corpus, repo_id).await {
            let (record, changed) = self.reconcile_repo_corpus_record_for_reads(record);
            self.repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((corpus, repo_id.to_string()), record.clone());
            if changed {
                self.cache.set_repo_corpus_record(&record).await;
            }
            return Some(record);
        }
        if let Some(record) = self
            .repo_corpus_snapshot_for_reads()
            .await
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            let (record, changed) = self.reconcile_repo_corpus_record_for_reads(record);
            self.repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((corpus, repo_id.to_string()), record.clone());
            if changed {
                self.cache.set_repo_corpus_record(&record).await;
            }
            return Some(record);
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn clear_in_memory_repo_runtime_for_test(&self, repo_id: &str) {
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
    }

    #[cfg(test)]
    pub(crate) fn clear_all_in_memory_repo_runtime_for_test(&self) {
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    #[cfg(test)]
    pub(crate) fn clear_all_in_memory_repo_corpus_records_for_test(&self) {
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    #[cfg(test)]
    pub(crate) async fn clear_persisted_repo_corpus_for_test(&self, repo_id: &str) {
        self.clear_in_memory_repo_runtime_for_test(repo_id);
        for corpus in [
            SearchCorpusKind::RepoEntity,
            SearchCorpusKind::RepoContentChunk,
        ] {
            self.cache.delete_repo_corpus_record(corpus, repo_id).await;
        }
        self.cache.delete_repo_corpus_snapshot().await;
    }

    #[cfg(test)]
    pub(crate) fn clear_in_memory_repo_publications_for_test(&self, repo_id: &str) {
        for record in self
            .repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values_mut()
        {
            if record.repo_id == repo_id {
                record.publication = None;
            }
        }
    }
}
