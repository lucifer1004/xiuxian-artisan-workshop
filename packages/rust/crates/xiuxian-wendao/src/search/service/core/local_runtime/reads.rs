use std::fs;

use crate::search::service::core::types::SearchPlaneService;
use crate::search::{SearchCorpusKind, SearchManifestRecord};

impl SearchPlaneService {
    pub(crate) fn load_local_corpus_manifest(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchManifestRecord> {
        let payload = fs::read(self.local_corpus_manifest_json_path(corpus)).ok()?;
        serde_json::from_slice(payload.as_slice()).ok()
    }
}
