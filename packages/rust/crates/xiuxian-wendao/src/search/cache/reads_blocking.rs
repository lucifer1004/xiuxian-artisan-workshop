use redis::Commands;
use serde::de::DeserializeOwned;

use crate::search::cache::SearchPlaneCache;
use crate::search::{
    SearchCorpusKind, SearchManifestRecord, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
};

impl SearchPlaneCache {
    fn blocking_connection(&self) -> Option<redis::Connection> {
        let client = self.client.as_ref()?;
        let connection = client
            .get_connection_with_timeout(self.config.connection_timeout)
            .ok()?;
        let _ = connection.set_read_timeout(Some(self.config.response_timeout));
        let _ = connection.set_write_timeout(Some(self.config.response_timeout));
        Some(connection)
    }

    fn get_json_blocking<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let mut connection = self.blocking_connection()?;
        let payload: Option<String> = connection.get(key).ok()?;
        serde_json::from_str(payload?.as_str()).ok()
    }

    pub(crate) fn get_repo_corpus_record_blocking(
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
        self.get_json_blocking(key.as_str())
    }

    pub(crate) fn get_repo_corpus_snapshot_blocking(
        &self,
    ) -> Option<SearchRepoCorpusSnapshotRecord> {
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
        self.get_json_blocking(key.as_str())
    }

    pub(crate) fn get_corpus_manifest_blocking(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchManifestRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .corpus_manifests
            .get(&corpus)
            .cloned()
        {
            return Some(record);
        }
        let key = self.keyspace.corpus_manifest_key(corpus);
        self.get_json_blocking(key.as_str())
    }
}
