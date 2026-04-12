use std::collections::BTreeMap;

use redis::AsyncCommands;
use serde::de::DeserializeOwned;

use crate::search::cache::SearchPlaneCache;
use crate::search::{
    SearchCorpusKind, SearchFileFingerprint, SearchRepoCorpusRecord,
    SearchRepoCorpusSnapshotRecord, SearchRepoPublicationRecord,
};

impl SearchPlaneCache {
    pub(crate) async fn get_json<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        #[cfg(test)]
        if let Some(payload) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .generic_json_payloads
            .get(key)
            .cloned()
        {
            return serde_json::from_str(payload.as_str()).ok();
        }
        let client = self.client.as_ref()?;
        let mut connection = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
            .ok()?;
        let payload: Option<String> = connection.get(key).await.ok()?;
        serde_json::from_str(payload?.as_str()).ok()
    }

    pub(crate) async fn get_repo_corpus_record(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_corpus_record_key(corpus, repo_id);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_publication_for_revision(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        revision: &str,
    ) -> Option<SearchRepoPublicationRecord> {
        let normalized_revision = revision.trim().to_ascii_lowercase();
        if normalized_revision.is_empty() {
            return None;
        }
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_publications_by_revision
            .get(&(corpus, repo_id.to_string(), normalized_revision.clone()))
            .cloned()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_publication_revision_key(
            corpus,
            repo_id,
            normalized_revision.as_str(),
        );
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_publication_revisions(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Vec<String> {
        #[cfg(test)]
        if let Some(revisions) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_publication_revision_indexes
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return revisions;
        }
        let Some(client) = self.client.as_ref() else {
            return Vec::new();
        };
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return Vec::new();
        };
        let key = self
            .keyspace
            .repo_publication_revision_index_key(corpus, repo_id);
        connection.lrange(key, 0, -1).await.unwrap_or_default()
    }

    pub(crate) async fn get_repo_corpus_snapshot(&self) -> Option<SearchRepoCorpusSnapshotRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_snapshot
            .clone()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_corpus_snapshot_key();
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<BTreeMap<String, SearchFileFingerprint>> {
        #[cfg(test)]
        if let Some(fingerprints) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .corpus_file_fingerprints
            .get(&corpus)
            .cloned()
        {
            return Some(fingerprints);
        }
        let key = self.keyspace.corpus_file_fingerprints_key(corpus);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<BTreeMap<String, SearchFileFingerprint>> {
        #[cfg(test)]
        if let Some(fingerprints) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_file_fingerprints
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return Some(fingerprints);
        }
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        self.get_json(key.as_str()).await
    }
}
