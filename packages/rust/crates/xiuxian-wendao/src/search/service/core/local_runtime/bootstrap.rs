use crate::search::service::core::types::SearchPlaneService;
use crate::search::{SearchCorpusKind, SearchCorpusStatus, SearchManifestRecord};

const LOCAL_BOOTSTRAP_CORPORA: [SearchCorpusKind; 4] = [
    SearchCorpusKind::KnowledgeSection,
    SearchCorpusKind::Attachment,
    SearchCorpusKind::LocalSymbol,
    SearchCorpusKind::ReferenceOccurrence,
];

impl SearchPlaneService {
    pub(crate) fn restore_local_corpus_statuses_from_runtime(&self) {
        for corpus in LOCAL_BOOTSTRAP_CORPORA {
            let Some(status) = self.local_corpus_status_for_bootstrap(corpus) else {
                continue;
            };
            self.coordinator.replace_status(status);
        }
    }

    fn local_corpus_status_for_bootstrap(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchCorpusStatus> {
        let manifest = self.local_corpus_manifest_for_bootstrap(corpus)?;
        let status = manifest.to_status()?;
        let active_epoch = status.active_epoch?;
        self.local_epoch_is_query_readable(corpus, active_epoch)
            .then_some(status)
    }

    fn local_corpus_manifest_for_bootstrap(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchManifestRecord> {
        self.cache
            .get_corpus_manifest_blocking(corpus)
            .or_else(|| self.load_local_corpus_manifest(corpus))
    }

    fn local_epoch_is_query_readable(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        !self
            .local_epoch_table_names_for_reads(corpus, epoch)
            .is_empty()
    }
}
