use std::collections::{BTreeMap, BTreeSet};

use crate::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search::service::core::types::{RepoRuntimeState, SearchPlaneService};
use crate::search::{
    SearchCorpusKind, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
    SearchRepoPublicationRecord,
};

impl SearchPlaneService {
    pub(crate) fn repo_index_bootstrap_statuses(
        &self,
        repo_ids: &[String],
    ) -> BTreeMap<String, RepoIndexEntryStatus> {
        let records = self.repo_corpus_snapshot_for_bootstrap(repo_ids);
        repo_ids
            .iter()
            .filter_map(|repo_id| {
                let entity_record = records.get(&(SearchCorpusKind::RepoEntity, repo_id.clone()));
                let content_record =
                    records.get(&(SearchCorpusKind::RepoContentChunk, repo_id.clone()));
                Self::repo_index_bootstrap_status_from_records(
                    repo_id.as_str(),
                    entity_record,
                    content_record,
                )
                .map(|status| (repo_id.clone(), status))
            })
            .collect()
    }

    fn repo_corpus_snapshot_for_bootstrap(
        &self,
        repo_ids: &[String],
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        let repo_ids = repo_ids.iter().cloned().collect::<BTreeSet<_>>();
        let current = self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if !current.is_empty() {
            return Self::filter_repo_corpus_records(current, &repo_ids);
        }
        if let Some(snapshot) = self.cache.get_repo_corpus_snapshot_blocking() {
            return Self::filter_repo_corpus_snapshot(snapshot, &repo_ids);
        }
        if let Some(snapshot) = self.load_local_repo_corpus_snapshot() {
            return Self::filter_repo_corpus_snapshot(snapshot, &repo_ids);
        }
        let cached_records = self.load_cached_repo_corpus_records_for_bootstrap(&repo_ids);
        if !cached_records.is_empty() {
            return cached_records;
        }
        self.load_local_repo_corpus_records_for_bootstrap(&repo_ids)
    }

    fn filter_repo_corpus_records(
        records: BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord>,
        repo_ids: &BTreeSet<String>,
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        if repo_ids.is_empty() {
            return records;
        }
        records
            .into_iter()
            .filter(|(_, record)| repo_ids.contains(&record.repo_id))
            .collect()
    }

    fn filter_repo_corpus_snapshot(
        snapshot: SearchRepoCorpusSnapshotRecord,
        repo_ids: &BTreeSet<String>,
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        snapshot
            .records
            .into_iter()
            .filter(|record| repo_ids.is_empty() || repo_ids.contains(&record.repo_id))
            .map(|record| ((record.corpus, record.repo_id.clone()), record))
            .collect()
    }

    fn load_local_repo_corpus_records_for_bootstrap(
        &self,
        repo_ids: &BTreeSet<String>,
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        let mut records = BTreeMap::new();
        for repo_id in repo_ids {
            for corpus in [
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ] {
                if let Some(record) = self.load_local_repo_corpus_record(corpus, repo_id.as_str()) {
                    records.insert((corpus, repo_id.clone()), record);
                }
            }
        }
        records
    }

    fn load_cached_repo_corpus_records_for_bootstrap(
        &self,
        repo_ids: &BTreeSet<String>,
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        let mut records = BTreeMap::new();
        for repo_id in repo_ids {
            for corpus in [
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ] {
                if let Some(record) = self
                    .cache
                    .get_repo_corpus_record_blocking(corpus, repo_id.as_str())
                {
                    records.insert((corpus, repo_id.clone()), record);
                }
            }
        }
        records
    }

    fn repo_index_bootstrap_status_from_records(
        repo_id: &str,
        entity_record: Option<&SearchRepoCorpusRecord>,
        content_record: Option<&SearchRepoCorpusRecord>,
    ) -> Option<RepoIndexEntryStatus> {
        let entity_publication = entity_record
            .and_then(|record| record.publication.as_ref())
            .filter(|publication| publication.is_parquet_query_readable());
        let content_publication = content_record
            .and_then(|record| record.publication.as_ref())
            .filter(|publication| publication.is_parquet_query_readable());
        let runtime = entity_record
            .and_then(|record| record.runtime.as_ref())
            .or_else(|| content_record.and_then(|record| record.runtime.as_ref()))
            .map(RepoRuntimeState::from_record);

        if entity_publication.is_some() && content_publication.is_some() {
            return Some(RepoIndexEntryStatus {
                repo_id: repo_id.to_string(),
                phase: RepoIndexPhase::Ready,
                queue_position: None,
                last_error: None,
                last_revision: runtime
                    .as_ref()
                    .and_then(|state| state.last_revision.clone())
                    .or_else(|| {
                        Self::bootstrap_publication_revision(
                            entity_publication,
                            content_publication,
                        )
                    }),
                updated_at: runtime
                    .as_ref()
                    .and_then(|state| state.updated_at.clone())
                    .or_else(|| {
                        Self::bootstrap_publication_timestamp(
                            entity_publication,
                            content_publication,
                        )
                    }),
                attempt_count: 0,
            });
        }

        runtime.and_then(|state| match state.phase {
            RepoIndexPhase::Unsupported | RepoIndexPhase::Failed => Some(state.as_status(repo_id)),
            _ => None,
        })
    }

    fn bootstrap_publication_revision(
        entity_publication: Option<&SearchRepoPublicationRecord>,
        content_publication: Option<&SearchRepoPublicationRecord>,
    ) -> Option<String> {
        entity_publication
            .and_then(|publication| publication.source_revision.clone())
            .or_else(|| {
                content_publication.and_then(|publication| publication.source_revision.clone())
            })
    }

    fn bootstrap_publication_timestamp(
        entity_publication: Option<&SearchRepoPublicationRecord>,
        content_publication: Option<&SearchRepoPublicationRecord>,
    ) -> Option<String> {
        entity_publication
            .map(|publication| publication.published_at.clone())
            .or_else(|| content_publication.map(|publication| publication.published_at.clone()))
    }
}
