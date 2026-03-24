use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use xiuxian_vector::{VectorStore, VectorStoreError};

use super::types::SearchPlaneService;
use crate::search_plane::service::helpers::{default_storage_root, manifest_keyspace_for_project};
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneCoordinator,
};

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
            maintenance_tasks: Arc::new(Mutex::new(std::collections::BTreeSet::new())),
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

    fn repo_table_name(corpus: SearchCorpusKind, repo_id: &str) -> String {
        format!(
            "{}_repo_{}",
            corpus.as_str(),
            blake3::hash(repo_id.as_bytes()).to_hex()
        )
    }
}
