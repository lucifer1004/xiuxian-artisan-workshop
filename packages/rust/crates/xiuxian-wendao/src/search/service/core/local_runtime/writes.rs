use std::fs;

use crate::search::service::core::types::SearchPlaneService;
use crate::search::{SearchCorpusStatus, SearchManifestRecord};

impl SearchPlaneService {
    pub(crate) async fn persist_local_corpus_manifest_status(&self, status: &SearchCorpusStatus) {
        if status.corpus.is_repo_backed() || status.active_epoch.is_none() {
            return;
        }

        let record = SearchManifestRecord::from_status(status);
        self.persist_local_corpus_manifest(&record);
        self.cache.set_corpus_manifest(&record).await;
    }

    pub(crate) fn persist_local_corpus_manifest(&self, record: &SearchManifestRecord) {
        let path = self.local_corpus_manifest_json_path(record.corpus);
        let Some(parent) = path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(payload) = serde_json::to_vec(record) else {
            return;
        };
        let _ = fs::write(path, payload);
    }
}
