use crate::search_plane::SearchCorpusKind;
use crate::search_plane::service::core::types::SearchPlaneService;

impl SearchPlaneService {
    #[must_use]
    pub(crate) fn corpus_root(&self, corpus: SearchCorpusKind) -> std::path::PathBuf {
        self.storage_root.join(corpus.as_str())
    }

    /// Table name for a published or staging epoch.
    #[must_use]
    pub(crate) fn table_name(corpus: SearchCorpusKind, epoch: u64) -> String {
        format!("{}_epoch_{epoch}", corpus.as_str())
    }

    #[must_use]
    pub(crate) fn local_partition_table_name(
        corpus: SearchCorpusKind,
        epoch: u64,
        partition_id: &str,
    ) -> String {
        format!("{}_epoch_{epoch}_part_{partition_id}", corpus.as_str())
    }

    #[must_use]
    pub(crate) fn local_epoch_table_names_for_reads(
        &self,
        corpus: SearchCorpusKind,
        epoch: u64,
    ) -> Vec<String> {
        let mut table_names = self.local_epoch_partition_table_names(corpus, epoch);
        if !table_names.is_empty() {
            table_names.sort();
            return table_names;
        }

        let legacy_table_name = Self::table_name(corpus, epoch);
        if self.local_table_exists(corpus, legacy_table_name.as_str()) {
            table_names.push(legacy_table_name);
        }
        table_names
    }

    #[must_use]
    pub(crate) fn local_epoch_has_partition_tables(
        &self,
        corpus: SearchCorpusKind,
        epoch: u64,
    ) -> bool {
        !self
            .local_epoch_partition_table_names(corpus, epoch)
            .is_empty()
    }

    #[must_use]
    pub(crate) fn local_table_exists(&self, corpus: SearchCorpusKind, table_name: &str) -> bool {
        self.corpus_root(corpus)
            .join(format!("{table_name}.lance"))
            .exists()
    }

    #[must_use]
    pub(crate) fn repo_content_chunk_table_name(repo_id: &str) -> String {
        Self::repo_table_name(SearchCorpusKind::RepoContentChunk, repo_id)
    }

    #[must_use]
    pub(crate) fn repo_entity_table_name(repo_id: &str) -> String {
        Self::repo_table_name(SearchCorpusKind::RepoEntity, repo_id)
    }

    fn repo_table_name(corpus: SearchCorpusKind, repo_id: &str) -> String {
        format!(
            "{}_repo_{}",
            corpus.as_str(),
            blake3::hash(repo_id.as_bytes()).to_hex()
        )
    }

    fn local_epoch_partition_table_names(
        &self,
        corpus: SearchCorpusKind,
        epoch: u64,
    ) -> Vec<String> {
        let root = self.corpus_root(corpus);
        let prefix = format!("{}_epoch_{epoch}_part_", corpus.as_str());
        let Ok(entries) = std::fs::read_dir(root) else {
            return Vec::new();
        };

        entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let Ok(file_type) = entry.file_type() else {
                    return None;
                };
                if !file_type.is_dir() {
                    return None;
                }

                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                let table_name = file_name.strip_suffix(".lance")?;
                table_name
                    .starts_with(prefix.as_str())
                    .then(|| table_name.to_string())
            })
            .collect()
    }
}
