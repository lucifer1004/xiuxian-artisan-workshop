use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use xiuxian_vector::{VectorStore, VectorStoreError};

use super::types::SearchPlaneService;
use crate::search_plane::service::helpers::{default_storage_root, manifest_keyspace_for_project};
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneCoordinator,
};

const DEFAULT_REPO_SEARCH_READ_CONCURRENCY_FALLBACK: usize = 8;
const MIN_REPO_SEARCH_READ_CONCURRENCY: usize = 4;
const MAX_REPO_SEARCH_READ_CONCURRENCY: usize = 16;
const REPO_SEARCH_READ_CONCURRENCY_ENV: &str = "XIUXIAN_WENDAO_REPO_SEARCH_READ_CONCURRENCY";

impl SearchPlaneService {
    /// Create a service rooted under project-local `PRJ_DATA_HOME`.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        let storage_root = default_storage_root(project_root.as_path());
        let manifest_keyspace = manifest_keyspace_for_project(project_root.as_path());
        let cache =
            crate::search_plane::cache::SearchPlaneCache::from_env(manifest_keyspace.clone());
        Self::with_runtime(
            project_root,
            storage_root,
            manifest_keyspace,
            SearchMaintenancePolicy::default(),
            cache,
        )
    }

    /// Create a service with explicit storage root, keyspace, and policy.
    #[must_use]
    pub fn with_paths(
        project_root: PathBuf,
        storage_root: PathBuf,
        manifest_keyspace: SearchManifestKeyspace,
        maintenance_policy: SearchMaintenancePolicy,
    ) -> Self {
        let cache =
            crate::search_plane::cache::SearchPlaneCache::disabled(manifest_keyspace.clone());
        Self::with_runtime(
            project_root,
            storage_root,
            manifest_keyspace,
            maintenance_policy,
            cache,
        )
    }

    pub(crate) fn with_runtime(
        project_root: PathBuf,
        storage_root: PathBuf,
        manifest_keyspace: SearchManifestKeyspace,
        maintenance_policy: SearchMaintenancePolicy,
        cache: crate::search_plane::cache::SearchPlaneCache,
    ) -> Self {
        let coordinator = Arc::new(SearchPlaneCoordinator::new(
            project_root.clone(),
            storage_root.clone(),
            manifest_keyspace.clone(),
            maintenance_policy,
        ));
        Self {
            project_root,
            storage_root,
            manifest_keyspace,
            coordinator,
            cache,
            repo_search_read_permits: Arc::new(
                Semaphore::new(repo_search_read_concurrency_limit()),
            ),
            local_maintenance: Arc::new(Mutex::new(
                super::types::LocalMaintenanceRuntime::default(),
            )),
            repo_maintenance: Arc::new(Mutex::new(super::types::RepoMaintenanceRuntime::default())),
            query_telemetry: Arc::new(RwLock::new(std::collections::BTreeMap::new())),
            repo_corpus_records: Arc::new(RwLock::new(std::collections::BTreeMap::new())),
        }
    }

    /// Absolute project root for this service.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Root directory that contains all per-corpus stores.
    #[must_use]
    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    /// Valkey namespace used by this service.
    #[must_use]
    pub fn manifest_keyspace(&self) -> &SearchManifestKeyspace {
        &self.manifest_keyspace
    }

    /// Shared coordinator for background build state.
    #[must_use]
    pub fn coordinator(&self) -> Arc<SearchPlaneCoordinator> {
        Arc::clone(&self.coordinator)
    }

    #[must_use]
    pub(crate) fn corpus_root(&self, corpus: SearchCorpusKind) -> PathBuf {
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

    /// Open the Lance-backed store for one search corpus.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot be initialized.
    pub async fn open_store(
        &self,
        corpus: SearchCorpusKind,
    ) -> Result<VectorStore, VectorStoreError> {
        let root = self.corpus_root(corpus);
        VectorStore::new(root.to_string_lossy().as_ref(), None).await
    }

    pub(crate) async fn acquire_repo_search_read_permit(
        &self,
    ) -> Result<OwnedSemaphorePermit, VectorStoreError> {
        Arc::clone(&self.repo_search_read_permits)
            .acquire_owned()
            .await
            .map_err(|_| {
                VectorStoreError::General(
                    "repo content search read permits are unavailable".to_string(),
                )
            })
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

fn repo_search_read_concurrency_limit() -> usize {
    repo_search_read_concurrency_limit_with_lookup(
        &|key| std::env::var(key).ok(),
        std::thread::available_parallelism()
            .ok()
            .map(std::num::NonZeroUsize::get),
    )
}

fn repo_search_read_concurrency_limit_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
    available_parallelism: Option<usize>,
) -> usize {
    lookup(REPO_SEARCH_READ_CONCURRENCY_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| default_repo_search_read_concurrency_limit(available_parallelism))
}

fn default_repo_search_read_concurrency_limit(available_parallelism: Option<usize>) -> usize {
    available_parallelism
        .unwrap_or(DEFAULT_REPO_SEARCH_READ_CONCURRENCY_FALLBACK)
        .div_ceil(2)
        .clamp(
            MIN_REPO_SEARCH_READ_CONCURRENCY,
            MAX_REPO_SEARCH_READ_CONCURRENCY,
        )
}

#[cfg(test)]
mod tests {
    use super::repo_search_read_concurrency_limit_with_lookup;

    #[test]
    fn repo_search_read_concurrency_limit_defaults_from_parallelism() {
        let limit = repo_search_read_concurrency_limit_with_lookup(&|_| None, Some(12));
        assert_eq!(limit, 6);
    }

    #[test]
    fn repo_search_read_concurrency_limit_accepts_positive_override() {
        let limit = repo_search_read_concurrency_limit_with_lookup(
            &|key| (key == "XIUXIAN_WENDAO_REPO_SEARCH_READ_CONCURRENCY").then(|| "9".to_string()),
            Some(12),
        );
        assert_eq!(limit, 9);
    }

    #[test]
    fn repo_search_read_concurrency_limit_ignores_invalid_override() {
        let limit = repo_search_read_concurrency_limit_with_lookup(
            &|key| {
                (key == "XIUXIAN_WENDAO_REPO_SEARCH_READ_CONCURRENCY")
                    .then(|| "invalid".to_string())
            },
            Some(6),
        );
        assert_eq!(limit, 4);
    }
}
