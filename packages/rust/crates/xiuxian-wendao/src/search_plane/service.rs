use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use xiuxian_io::PrjDirs;
use xiuxian_vector::{TableInfo, VectorStore, VectorStoreError};

use super::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusIssueSummary,
    SearchCorpusKind, SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
    SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchMaintenancePolicy,
    SearchManifestKeyspace, SearchPlaneCacheTtl, SearchPlaneCoordinator, SearchPlanePhase,
    SearchPlaneStatusSnapshot, SearchRepoManifestRecord, attachment, cache::SearchPlaneCache,
    coordinator::SearchCompactionTask, knowledge_section, local_symbol, reference_occurrence,
    repo_content_chunk, repo_entity,
};
use crate::gateway::studio::repo_index::{
    RepoIndexEntryStatus, RepoIndexPhase, RepoIndexStatusResponse,
};

/// Project-scoped entrypoint for the search-plane domain.
#[derive(Clone)]
pub struct SearchPlaneService {
    project_root: PathBuf,
    storage_root: PathBuf,
    manifest_keyspace: SearchManifestKeyspace,
    coordinator: Arc<SearchPlaneCoordinator>,
    cache: SearchPlaneCache,
    maintenance_tasks: Arc<Mutex<BTreeSet<SearchCorpusKind>>>,
    repo_publications: Arc<RwLock<BTreeMap<(SearchCorpusKind, String), SearchRepoManifestRecord>>>,
}

impl SearchPlaneService {
    /// Create a service rooted under project-local `PRJ_DATA_HOME`.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        let storage_root = default_storage_root(project_root.as_path());
        let manifest_keyspace = manifest_keyspace_for_project(project_root.as_path());
        let cache = SearchPlaneCache::from_env(manifest_keyspace.clone());
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
        let cache = SearchPlaneCache::disabled(manifest_keyspace.clone());
        Self::with_runtime(
            project_root,
            storage_root,
            manifest_keyspace,
            maintenance_policy,
            cache,
        )
    }

    fn with_runtime(
        project_root: PathBuf,
        storage_root: PathBuf,
        manifest_keyspace: SearchManifestKeyspace,
        maintenance_policy: SearchMaintenancePolicy,
        cache: SearchPlaneCache,
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
            maintenance_tasks: Arc::new(Mutex::new(BTreeSet::new())),
            repo_publications: Arc::new(RwLock::new(BTreeMap::new())),
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

    /// Snapshot current multi-corpus status.
    #[must_use]
    pub fn status(&self) -> SearchPlaneStatusSnapshot {
        let mut snapshot = self.coordinator.status();
        annotate_status_snapshot(&mut snapshot);
        snapshot
    }

    pub(crate) async fn status_with_repo_content(
        &self,
        repo_status: &RepoIndexStatusResponse,
    ) -> SearchPlaneStatusSnapshot {
        let mut snapshot = self.status();
        let repo_entity_status = self
            .synthesize_repo_table_status(repo_status, SearchCorpusKind::RepoEntity)
            .await;
        replace_corpus_status(&mut snapshot, repo_entity_status);
        let repo_content_status = self
            .synthesize_repo_table_status(repo_status, SearchCorpusKind::RepoContentChunk)
            .await;
        replace_corpus_status(&mut snapshot, repo_content_status);
        snapshot
    }

    /// Filesystem root for one corpus store.
    #[must_use]
    pub fn corpus_root(&self, corpus: SearchCorpusKind) -> PathBuf {
        self.storage_root.join(corpus.as_str())
    }

    /// Table name for a published or staging epoch.
    #[must_use]
    pub fn table_name(&self, corpus: SearchCorpusKind, epoch: u64) -> String {
        format!("{}_epoch_{epoch}", corpus.as_str())
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

    pub(crate) fn ensure_local_symbol_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
    ) {
        local_symbol::ensure_local_symbol_index_started(self, project_root, config_root, projects);
    }

    pub(crate) async fn search_local_symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::AstSearchHit>,
        local_symbol::LocalSymbolSearchError,
    > {
        local_symbol::search_local_symbols(self, query, limit).await
    }

    pub(crate) fn ensure_knowledge_section_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
    ) {
        knowledge_section::ensure_knowledge_section_index_started(
            self,
            project_root,
            config_root,
            projects,
        );
    }

    pub(crate) async fn search_knowledge_sections(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::SearchHit>,
        knowledge_section::KnowledgeSectionSearchError,
    > {
        knowledge_section::search_knowledge_sections(self, query, limit).await
    }

    pub(crate) fn ensure_attachment_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
    ) {
        attachment::ensure_attachment_index_started(self, project_root, config_root, projects);
    }

    pub(crate) async fn search_attachment_hits(
        &self,
        query: &str,
        limit: usize,
        extensions: &[String],
        kinds: &[crate::link_graph::LinkGraphAttachmentKind],
        case_sensitive: bool,
    ) -> Result<
        Vec<crate::gateway::studio::types::AttachmentSearchHit>,
        attachment::AttachmentSearchError,
    > {
        attachment::search_attachment_hits(self, query, limit, extensions, kinds, case_sensitive)
            .await
    }

    pub(crate) async fn autocomplete_local_symbols(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::AutocompleteSuggestion>,
        local_symbol::LocalSymbolSearchError,
    > {
        local_symbol::autocomplete_local_symbols(self, prefix, limit).await
    }

    pub(crate) fn ensure_reference_occurrence_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
    ) {
        reference_occurrence::ensure_reference_occurrence_index_started(
            self,
            project_root,
            config_root,
            projects,
        );
    }

    pub(crate) async fn search_reference_occurrences(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::ReferenceSearchHit>,
        reference_occurrence::ReferenceOccurrenceSearchError,
    > {
        reference_occurrence::search_reference_occurrences(self, query, limit).await
    }

    #[must_use]
    pub(crate) fn repo_content_chunk_table_name(&self, repo_id: &str) -> String {
        self.repo_table_name(SearchCorpusKind::RepoContentChunk, repo_id)
    }

    #[must_use]
    pub(crate) fn repo_entity_table_name(&self, repo_id: &str) -> String {
        self.repo_table_name(SearchCorpusKind::RepoEntity, repo_id)
    }

    pub(crate) async fn publish_repo_content_chunks_with_revision(
        &self,
        repo_id: &str,
        documents: &[crate::gateway::studio::repo_index::RepoCodeDocument],
        source_revision: Option<&str>,
    ) -> Result<(), xiuxian_vector::VectorStoreError> {
        repo_content_chunk::publish_repo_content_chunks(self, repo_id, documents, source_revision)
            .await
    }

    pub(crate) async fn search_repo_content_chunks(
        &self,
        repo_id: &str,
        search_term: &str,
        language_filters: &std::collections::HashSet<String>,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::SearchHit>,
        repo_content_chunk::RepoContentChunkSearchError,
    > {
        repo_content_chunk::search_repo_content_chunks(
            self,
            repo_id,
            search_term,
            language_filters,
            limit,
        )
        .await
    }

    pub(crate) async fn publish_repo_entities_with_revision(
        &self,
        repo_id: &str,
        analysis: &crate::analyzers::RepositoryAnalysisOutput,
        source_revision: Option<&str>,
    ) -> Result<(), xiuxian_vector::VectorStoreError> {
        repo_entity::publish_repo_entities(self, repo_id, analysis, source_revision).await
    }

    pub(crate) async fn record_repo_publication(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        table_name: &str,
        source_revision: Option<&str>,
        table_info: &TableInfo,
    ) {
        let record = SearchRepoManifestRecord::new(
            corpus,
            repo_id,
            table_name,
            corpus.schema_version(),
            source_revision,
            table_info.version_id,
            table_info.num_rows,
            u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
            table_info.commit_timestamp.as_str(),
        );
        self.repo_publications
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((corpus, repo_id.to_string()), record.clone());
        self.cache.set_repo_manifest(&record).await;
    }

    pub(crate) fn clear_repo_publications(&self, repo_id: &str) {
        self.repo_publications
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let cache = self.cache.clone();
            let repo_id = repo_id.to_string();
            handle.spawn(async move {
                cache
                    .delete_repo_manifest(SearchCorpusKind::RepoEntity, repo_id.as_str())
                    .await;
                cache
                    .delete_repo_manifest(SearchCorpusKind::RepoContentChunk, repo_id.as_str())
                    .await;
            });
        }
    }

    pub(crate) async fn has_published_repo_corpus(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> bool {
        self.repo_publication_for_reads(corpus, repo_id)
            .await
            .is_some()
    }

    pub(crate) async fn search_repo_entities(
        &self,
        repo_id: &str,
        search_term: &str,
        language_filters: &std::collections::HashSet<String>,
        kind_filters: &std::collections::HashSet<String>,
        limit: usize,
    ) -> Result<Vec<crate::gateway::studio::types::SearchHit>, repo_entity::RepoEntitySearchError>
    {
        repo_entity::search_repo_entities(
            self,
            repo_id,
            search_term,
            language_filters,
            kind_filters,
            limit,
        )
        .await
    }

    #[must_use]
    pub(crate) fn corpus_active_epoch(&self, corpus: SearchCorpusKind) -> Option<u64> {
        self.coordinator.status_for(corpus).active_epoch
    }

    #[must_use]
    pub(crate) fn autocomplete_cache_key(&self, prefix: &str, limit: usize) -> Option<String> {
        let epoch = self.corpus_active_epoch(SearchCorpusKind::LocalSymbol)?;
        self.cache.autocomplete_cache_key(prefix, limit, epoch)
    }

    #[must_use]
    pub(crate) fn search_query_cache_key(
        &self,
        scope: &str,
        corpora: &[SearchCorpusKind],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        let epochs = corpora
            .iter()
            .map(|corpus| {
                self.corpus_active_epoch(*corpus)
                    .map(|epoch| (*corpus, epoch))
            })
            .collect::<Option<Vec<_>>>()?;
        self.cache
            .search_query_cache_key(scope, epochs.as_slice(), query, limit, intent, repo_hint)
    }

    #[must_use]
    pub(crate) async fn repo_search_query_cache_key(
        &self,
        scope: &str,
        corpora: &[SearchCorpusKind],
        repo_corpora: &[SearchCorpusKind],
        repo_status: &RepoIndexStatusResponse,
        repo_ids: &[String],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        let mut versions = corpora
            .iter()
            .map(|corpus| self.corpus_cache_version(*corpus))
            .collect::<Vec<_>>();
        let repo_lookup = repo_status
            .repos
            .iter()
            .map(|status| (status.repo_id.as_str(), status))
            .collect::<std::collections::HashMap<_, _>>();
        let mut sorted_repo_ids = repo_ids.to_vec();
        sorted_repo_ids.sort_unstable();
        sorted_repo_ids.dedup();
        if sorted_repo_ids.is_empty() {
            versions.push("repo_set:none".to_string());
        }
        for repo_id in sorted_repo_ids {
            let status = repo_lookup.get(repo_id.as_str()).copied();
            for corpus in repo_corpora {
                if let Some(publication) = self
                    .repo_publication_for_reads(*corpus, repo_id.as_str())
                    .await
                {
                    versions.push(repo_publication_cache_version(status, &publication));
                } else {
                    versions.push(repo_corpus_cache_version(*corpus, repo_id.as_str(), status));
                }
            }
        }
        self.cache.search_query_cache_key_from_versions(
            scope,
            versions.as_slice(),
            query,
            limit,
            intent,
            repo_hint,
        )
    }

    pub(crate) async fn cache_get_json<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.cache.get_json(key).await
    }

    pub(crate) async fn cache_set_json<T>(&self, key: &str, ttl: SearchPlaneCacheTtl, value: &T)
    where
        T: serde::Serialize,
    {
        self.cache.set_json(key, ttl, value).await;
    }

    pub(crate) fn publish_ready_and_maintain(
        &self,
        lease: &super::SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        if !self
            .coordinator
            .publish_ready(lease, row_count, fragment_count)
        {
            return false;
        }
        self.schedule_pending_compaction(lease.corpus);
        true
    }

    fn schedule_pending_compaction(&self, corpus: SearchCorpusKind) {
        let Some(task) = self.coordinator.pending_compaction_task(corpus) else {
            return;
        };
        {
            let mut guard = self
                .maintenance_tasks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !guard.insert(corpus) {
                return;
            }
        }

        let service = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                service.run_compaction_task(task).await;
                service
                    .maintenance_tasks
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&corpus);
            });
        } else {
            self.maintenance_tasks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&corpus);
        }
    }

    async fn run_compaction_task(&self, task: SearchCompactionTask) {
        let table_name = self.table_name(task.corpus, task.active_epoch);
        let store = match self.open_store(task.corpus).await {
            Ok(store) => store,
            Err(error) => {
                log::warn!(
                    "search-plane compaction failed to open store for {} epoch {}: {}",
                    task.corpus,
                    task.active_epoch,
                    error
                );
                return;
            }
        };
        match store.compact(table_name.as_str()).await {
            Ok(stats) => {
                let fragment_count = u64::try_from(stats.fragments_after).unwrap_or(u64::MAX);
                let _ = self.coordinator.mark_compaction_complete(
                    task.corpus,
                    task.active_epoch,
                    task.row_count,
                    fragment_count,
                    task.reason,
                );
            }
            Err(error) => {
                log::warn!(
                    "search-plane compaction failed for {} epoch {} table {}: {}",
                    task.corpus,
                    task.active_epoch,
                    table_name,
                    error
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn publish_local_symbol_hits(
        &self,
        fingerprint: &str,
        hits: &[crate::gateway::studio::types::AstSearchHit],
    ) -> Result<(), local_symbol::LocalSymbolBuildError> {
        local_symbol::publish_local_symbol_hits(self, fingerprint, hits).await
    }

    #[cfg(test)]
    pub(crate) async fn publish_reference_occurrences_from_projects(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
        fingerprint: &str,
    ) -> Result<(), reference_occurrence::ReferenceOccurrenceBuildError> {
        reference_occurrence::publish_reference_occurrences_from_projects(
            self,
            project_root,
            config_root,
            projects,
            fingerprint,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn publish_attachments_from_projects(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
        fingerprint: &str,
    ) -> Result<(), attachment::AttachmentBuildError> {
        attachment::publish_attachments_from_projects(
            self,
            project_root,
            config_root,
            projects,
            fingerprint,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn publish_knowledge_sections_from_projects(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[crate::gateway::studio::types::UiProjectConfig],
        fingerprint: &str,
    ) -> Result<(), knowledge_section::KnowledgeSectionBuildError> {
        knowledge_section::publish_knowledge_sections_from_projects(
            self,
            project_root,
            config_root,
            projects,
            fingerprint,
        )
        .await
    }

    fn repo_table_name(&self, corpus: SearchCorpusKind, repo_id: &str) -> String {
        format!(
            "{}_repo_{}",
            corpus.as_str(),
            blake3::hash(repo_id.as_bytes()).to_hex()
        )
    }

    async fn synthesize_repo_table_status(
        &self,
        repo_status: &RepoIndexStatusResponse,
        corpus: SearchCorpusKind,
    ) -> SearchCorpusStatus {
        let mut status = SearchCorpusStatus::new(corpus);
        let mut published_repos = Vec::new();
        let mut issues = Vec::new();
        let mut has_active_work = false;

        for repo in &repo_status.repos {
            update_latest_timestamp(&mut status.updated_at, repo.updated_at.as_deref());
            let publication = self
                .repo_publication_for_status(corpus, repo.repo_id.as_str())
                .await;
            if let Some(publication) = publication.as_ref() {
                if let Some(issue) = repo_publication_consistency_issue(corpus, repo, &publication)
                {
                    issues.push(issue);
                }
                published_repos.push((repo, publication.clone()));
            } else if matches!(repo.phase, RepoIndexPhase::Ready) {
                issues.push(repo_manifest_missing_issue(corpus, repo));
            }
            match repo.phase {
                RepoIndexPhase::Queued
                | RepoIndexPhase::Checking
                | RepoIndexPhase::Syncing
                | RepoIndexPhase::Indexing => {
                    has_active_work = true;
                }
                RepoIndexPhase::Failed => {
                    if let Some(issue) = repo_index_failure_issue(repo, publication.as_ref()) {
                        issues.push(issue);
                    }
                }
                RepoIndexPhase::Idle | RepoIndexPhase::Unsupported | RepoIndexPhase::Ready => {}
            }
        }

        if published_repos.is_empty() {
            status.phase = repo_content_phase(false, has_active_work, !issues.is_empty());
            status.last_error = join_issue_messages(&issues);
            status.issue_summary = summarize_issues(&issues);
            status.issues = issues;
            annotate_status_reason(&mut status);
            return status;
        }

        let mut ready_tables = 0usize;
        let mut row_count = 0u64;
        let mut fragment_count = 0u64;
        let mut fingerprint_parts = Vec::new();

        for (repo, publication) in published_repos {
            ready_tables = ready_tables.saturating_add(1);
            row_count = row_count.saturating_add(publication.row_count);
            fragment_count = fragment_count.saturating_add(publication.fragment_count);
            update_latest_timestamp(
                &mut status.build_finished_at,
                Some(publication.published_at.as_str()),
            );
            update_latest_timestamp(
                &mut status.updated_at,
                Some(publication.published_at.as_str()),
            );
            fingerprint_parts.push(repo_corpus_fingerprint_part(repo, &publication));
        }

        let has_ready_tables = ready_tables > 0;
        status.phase = repo_content_phase(has_ready_tables, has_active_work, !issues.is_empty());
        if has_ready_tables {
            fingerprint_parts.sort_unstable();
            status.row_count = Some(row_count);
            status.fragment_count = Some(fragment_count);
            status.fingerprint = Some(
                blake3::hash(fingerprint_parts.join("|").as_bytes())
                    .to_hex()
                    .to_string(),
            );
        }
        status.last_error = join_issue_messages(&issues);
        status.issue_summary = summarize_issues(&issues);
        status.issues = issues;
        annotate_status_reason(&mut status);
        status
    }

    async fn repo_publication_for_status(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoManifestRecord> {
        self.repo_publication_for_reads(corpus, repo_id).await
    }

    fn cached_repo_publication(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoManifestRecord> {
        self.repo_publications
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(corpus, repo_id.to_string()))
            .cloned()
    }

    async fn repo_publication_for_reads(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoManifestRecord> {
        if let Some(record) = self.cached_repo_publication(corpus, repo_id) {
            return Some(record);
        }
        let record = self.cache.get_repo_manifest(corpus, repo_id).await?;
        if record.schema_version != corpus.schema_version() {
            return None;
        }
        self.repo_publications
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((corpus, repo_id.to_string()), record.clone());
        Some(record)
    }
}

fn default_storage_root(project_root: &Path) -> PathBuf {
    PrjDirs::data_home()
        .join("wendao")
        .join("search_plane")
        .join(project_hash(project_root))
}

fn manifest_keyspace_for_project(project_root: &Path) -> SearchManifestKeyspace {
    SearchManifestKeyspace::new(format!(
        "xiuxian:wendao:search_plane:{}",
        project_hash(project_root)
    ))
}

fn project_hash(project_root: &Path) -> String {
    blake3::hash(project_root.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

fn replace_corpus_status(snapshot: &mut SearchPlaneStatusSnapshot, status: SearchCorpusStatus) {
    if let Some(current) = snapshot
        .corpora
        .iter_mut()
        .find(|current| current.corpus == status.corpus)
    {
        *current = status;
        return;
    }
    snapshot.corpora.push(status);
    snapshot.corpora.sort_by_key(|entry| entry.corpus);
}

fn repo_content_phase(
    has_ready_tables: bool,
    has_active_work: bool,
    has_failures: bool,
) -> SearchPlanePhase {
    if has_active_work {
        return SearchPlanePhase::Indexing;
    }
    if has_ready_tables && has_failures {
        return SearchPlanePhase::Degraded;
    }
    if has_ready_tables {
        return SearchPlanePhase::Ready;
    }
    if has_failures {
        return SearchPlanePhase::Failed;
    }
    SearchPlanePhase::Idle
}

fn update_latest_timestamp(target: &mut Option<String>, candidate: Option<&str>) {
    let Some(candidate) = candidate else {
        return;
    };
    if target.as_deref().is_none_or(|current| current < candidate) {
        *target = Some(candidate.to_string());
    }
}

fn annotate_status_snapshot(snapshot: &mut SearchPlaneStatusSnapshot) {
    for status in &mut snapshot.corpora {
        annotate_status_reason(status);
    }
}

fn annotate_status_reason(status: &mut SearchCorpusStatus) {
    status.status_reason = derive_status_reason(status);
}

fn join_issue_messages(issues: &[SearchCorpusIssue]) -> Option<String> {
    if issues.is_empty() {
        return None;
    }
    Some(
        issues
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn derive_status_reason(status: &SearchCorpusStatus) -> Option<SearchCorpusStatusReason> {
    if let Some(summary) = status.issue_summary.as_ref() {
        let readable = status_is_readable(status);
        return Some(SearchCorpusStatusReason {
            code: reason_code_for_issue(summary.primary_code),
            severity: reason_severity_for_issue(summary.primary_code, readable),
            action: reason_action_for_issue(summary.primary_code),
            readable,
        });
    }

    match status.phase {
        SearchPlanePhase::Indexing => Some(SearchCorpusStatusReason {
            code: if status_is_readable(status) {
                SearchCorpusStatusReasonCode::Refreshing
            } else {
                SearchCorpusStatusReasonCode::WarmingUp
            },
            severity: SearchCorpusStatusSeverity::Info,
            action: SearchCorpusStatusAction::Wait,
            readable: status_is_readable(status),
        }),
        SearchPlanePhase::Failed => {
            let readable = status_is_readable(status);
            Some(SearchCorpusStatusReason {
                code: SearchCorpusStatusReasonCode::BuildFailed,
                severity: if readable {
                    SearchCorpusStatusSeverity::Warning
                } else {
                    SearchCorpusStatusSeverity::Error
                },
                action: SearchCorpusStatusAction::RetryBuild,
                readable,
            })
        }
        SearchPlanePhase::Ready => {
            status
                .maintenance
                .compaction_pending
                .then_some(SearchCorpusStatusReason {
                    code: SearchCorpusStatusReasonCode::CompactionPending,
                    severity: SearchCorpusStatusSeverity::Info,
                    action: SearchCorpusStatusAction::Wait,
                    readable: true,
                })
        }
        SearchPlanePhase::Idle | SearchPlanePhase::Degraded => None,
    }
}

fn summarize_issues(issues: &[SearchCorpusIssue]) -> Option<SearchCorpusIssueSummary> {
    let first = issues.first()?;
    let mut family = issue_family(first.code);
    let mut primary_code = first.code;
    let mut readable_issue_count = usize::from(first.readable);
    for issue in issues.iter().skip(1) {
        let current_family = issue_family(issue.code);
        if family != current_family {
            family = SearchCorpusIssueFamily::Mixed;
        }
        if issue_priority(issue.code) < issue_priority(primary_code) {
            primary_code = issue.code;
        }
        if issue.readable {
            readable_issue_count = readable_issue_count.saturating_add(1);
        }
    }
    Some(SearchCorpusIssueSummary {
        family,
        primary_code,
        issue_count: issues.len(),
        readable_issue_count,
    })
}

fn status_is_readable(status: &SearchCorpusStatus) -> bool {
    status.active_epoch.is_some()
        || status.row_count.is_some()
        || matches!(
            status.phase,
            SearchPlanePhase::Ready | SearchPlanePhase::Degraded
        )
}

fn reason_code_for_issue(code: SearchCorpusIssueCode) -> SearchCorpusStatusReasonCode {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing => {
            SearchCorpusStatusReasonCode::PublishedManifestMissing
        }
        SearchCorpusIssueCode::PublishedRevisionMissing => {
            SearchCorpusStatusReasonCode::PublishedRevisionMissing
        }
        SearchCorpusIssueCode::PublishedRevisionMismatch => {
            SearchCorpusStatusReasonCode::PublishedRevisionMismatch
        }
        SearchCorpusIssueCode::RepoIndexFailed => SearchCorpusStatusReasonCode::RepoIndexFailed,
    }
}

fn reason_action_for_issue(code: SearchCorpusIssueCode) -> SearchCorpusStatusAction {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing
        | SearchCorpusIssueCode::PublishedRevisionMissing
        | SearchCorpusIssueCode::PublishedRevisionMismatch => SearchCorpusStatusAction::ResyncRepo,
        SearchCorpusIssueCode::RepoIndexFailed => SearchCorpusStatusAction::InspectRepoSync,
    }
}

fn reason_severity_for_issue(
    code: SearchCorpusIssueCode,
    readable: bool,
) -> SearchCorpusStatusSeverity {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing => {
            if readable {
                SearchCorpusStatusSeverity::Warning
            } else {
                SearchCorpusStatusSeverity::Error
            }
        }
        SearchCorpusIssueCode::PublishedRevisionMissing
        | SearchCorpusIssueCode::PublishedRevisionMismatch => {
            if readable {
                SearchCorpusStatusSeverity::Warning
            } else {
                SearchCorpusStatusSeverity::Error
            }
        }
        SearchCorpusIssueCode::RepoIndexFailed => {
            if readable {
                SearchCorpusStatusSeverity::Warning
            } else {
                SearchCorpusStatusSeverity::Error
            }
        }
    }
}

fn issue_family(code: SearchCorpusIssueCode) -> SearchCorpusIssueFamily {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing
        | SearchCorpusIssueCode::PublishedRevisionMissing => SearchCorpusIssueFamily::Manifest,
        SearchCorpusIssueCode::PublishedRevisionMismatch => SearchCorpusIssueFamily::Revision,
        SearchCorpusIssueCode::RepoIndexFailed => SearchCorpusIssueFamily::RepoSync,
    }
}

fn issue_priority(code: SearchCorpusIssueCode) -> u8 {
    match code {
        SearchCorpusIssueCode::PublishedManifestMissing => 0,
        SearchCorpusIssueCode::PublishedRevisionMissing => 1,
        SearchCorpusIssueCode::PublishedRevisionMismatch => 2,
        SearchCorpusIssueCode::RepoIndexFailed => 3,
    }
}

fn repo_corpus_fingerprint_part(
    repo: &RepoIndexEntryStatus,
    publication: &SearchRepoManifestRecord,
) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        repo.repo_id,
        publication.source_revision.as_deref().unwrap_or_default(),
        repo_phase_cache_fragment(repo.phase),
        repo.last_revision.as_deref().unwrap_or_default(),
        publication.table_version_id,
        publication.row_count,
        publication.fragment_count
    )
}

impl SearchPlaneService {
    fn corpus_cache_version(&self, corpus: SearchCorpusKind) -> String {
        let status = self.coordinator.status_for(corpus);
        if let Some(epoch) = status.active_epoch {
            return format!("{corpus}:schema:{}:epoch:{epoch}", corpus.schema_version());
        }
        format!(
            "{corpus}:schema:{}:phase:{}",
            corpus.schema_version(),
            search_phase_cache_fragment(status.phase)
        )
    }
}

fn repo_corpus_cache_version(
    corpus: SearchCorpusKind,
    repo_id: &str,
    status: Option<&RepoIndexEntryStatus>,
) -> String {
    let Some(status) = status else {
        return format!(
            "{corpus}:schema:{}:repo:{}:phase:missing",
            corpus.schema_version(),
            normalize_cache_fragment(repo_id)
        );
    };
    format!(
        "{corpus}:schema:{}:repo:{}:phase:{}:revision:{}:updated:{}",
        corpus.schema_version(),
        normalize_cache_fragment(repo_id),
        repo_phase_cache_fragment(status.phase),
        normalize_cache_fragment(status.last_revision.as_deref().unwrap_or_default()),
        normalize_cache_fragment(status.updated_at.as_deref().unwrap_or_default())
    )
}

fn repo_publication_cache_version(
    status: Option<&RepoIndexEntryStatus>,
    publication: &SearchRepoManifestRecord,
) -> String {
    let base = publication.cache_version();
    let Some(status) = status else {
        return base;
    };
    let published_revision =
        normalize_cache_fragment(publication.source_revision.as_deref().unwrap_or_default());
    let current_revision =
        normalize_cache_fragment(status.last_revision.as_deref().unwrap_or_default());
    if status.phase == RepoIndexPhase::Ready
        && (current_revision.is_empty() || current_revision == published_revision)
    {
        return base;
    }
    format!(
        "{base}:phase:{}:current-revision:{current_revision}:published-revision:{published_revision}",
        repo_phase_cache_fragment(status.phase)
    )
}

fn repo_manifest_missing_issue(
    corpus: SearchCorpusKind,
    repo: &RepoIndexEntryStatus,
) -> SearchCorpusIssue {
    SearchCorpusIssue {
        code: SearchCorpusIssueCode::PublishedManifestMissing,
        readable: false,
        repo_id: Some(repo.repo_id.clone()),
        current_revision: repo.last_revision.clone(),
        published_revision: None,
        message: format!(
            "{}: published manifest for {} is missing",
            repo.repo_id,
            corpus.as_str()
        ),
    }
}

fn repo_index_failure_issue(
    repo: &RepoIndexEntryStatus,
    publication: Option<&SearchRepoManifestRecord>,
) -> Option<SearchCorpusIssue> {
    let message = repo.last_error.as_ref()?.clone();
    Some(SearchCorpusIssue {
        code: SearchCorpusIssueCode::RepoIndexFailed,
        readable: publication.is_some(),
        repo_id: Some(repo.repo_id.clone()),
        current_revision: repo.last_revision.clone(),
        published_revision: publication.and_then(|publication| publication.source_revision.clone()),
        message: format!("{}: {message}", repo.repo_id),
    })
}

fn repo_publication_consistency_issue(
    corpus: SearchCorpusKind,
    repo: &RepoIndexEntryStatus,
    publication: &SearchRepoManifestRecord,
) -> Option<SearchCorpusIssue> {
    if repo.phase != RepoIndexPhase::Ready {
        return None;
    }
    let current_revision = repo
        .last_revision
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    let published_revision = publication
        .source_revision
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if current_revision.is_empty() && published_revision.is_empty() {
        return None;
    }
    if published_revision.is_empty() {
        return Some(SearchCorpusIssue {
            code: SearchCorpusIssueCode::PublishedRevisionMissing,
            readable: true,
            repo_id: Some(repo.repo_id.clone()),
            current_revision: repo.last_revision.clone(),
            published_revision: publication.source_revision.clone(),
            message: format!(
                "{}: published manifest for {} is missing source revision while repo is ready at `{}`",
                repo.repo_id,
                corpus.as_str(),
                current_revision
            ),
        });
    }
    if current_revision.is_empty() || current_revision == published_revision {
        return None;
    }
    Some(SearchCorpusIssue {
        code: SearchCorpusIssueCode::PublishedRevisionMismatch,
        readable: true,
        repo_id: Some(repo.repo_id.clone()),
        current_revision: repo.last_revision.clone(),
        published_revision: publication.source_revision.clone(),
        message: format!(
            "{}: published manifest for {} targets revision `{published_revision}` but repo is ready at `{current_revision}`",
            repo.repo_id,
            corpus.as_str()
        ),
    })
}

fn search_phase_cache_fragment(phase: SearchPlanePhase) -> &'static str {
    match phase {
        SearchPlanePhase::Idle => "idle",
        SearchPlanePhase::Indexing => "indexing",
        SearchPlanePhase::Ready => "ready",
        SearchPlanePhase::Degraded => "degraded",
        SearchPlanePhase::Failed => "failed",
    }
}

fn repo_phase_cache_fragment(phase: RepoIndexPhase) -> &'static str {
    match phase {
        RepoIndexPhase::Idle => "idle",
        RepoIndexPhase::Queued => "queued",
        RepoIndexPhase::Checking => "checking",
        RepoIndexPhase::Syncing => "syncing",
        RepoIndexPhase::Indexing => "indexing",
        RepoIndexPhase::Ready => "ready",
        RepoIndexPhase::Unsupported => "unsupported",
        RepoIndexPhase::Failed => "failed",
    }
}

fn normalize_cache_fragment(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::analyzers::{
        ExampleRecord, ModuleRecord, RepoSymbolKind, RepositoryAnalysisOutput, SymbolRecord,
    };
    use crate::gateway::studio::repo_index::{
        RepoCodeDocument, RepoIndexEntryStatus, RepoIndexPhase, RepoIndexStatusResponse,
    };
    use crate::gateway::studio::types::{AstSearchHit, StudioNavigationTarget};

    #[tokio::test]
    async fn service_derives_stable_roots_and_opens_vector_store() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let manifest_keyspace = SearchManifestKeyspace::new("xiuxian:test:search_plane");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            manifest_keyspace.clone(),
            SearchMaintenancePolicy::default(),
        );

        assert_eq!(
            service.table_name(SearchCorpusKind::LocalSymbol, 7),
            "local_symbol_epoch_7"
        );
        assert_eq!(
            service
                .manifest_keyspace()
                .corpus_manifest_key(SearchCorpusKind::LocalSymbol),
            "xiuxian:test:search_plane:manifest:local_symbol"
        );

        let store = service
            .open_store(SearchCorpusKind::LocalSymbol)
            .await
            .expect("vector store should open");
        assert!(
            store
                .table_path(&service.table_name(SearchCorpusKind::LocalSymbol, 1))
                .starts_with(service.corpus_root(SearchCorpusKind::LocalSymbol))
        );
    }

    #[test]
    fn service_disables_cache_for_explicit_test_paths() {
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/project/.data/search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );

        assert!(service.autocomplete_cache_key("alpha", 8).is_none());
        assert!(
            service
                .search_query_cache_key(
                    "knowledge",
                    &[SearchCorpusKind::KnowledgeSection],
                    "alpha",
                    10,
                    Some("semantic_lookup"),
                    None,
                )
                .is_none()
        );
    }

    fn sample_hit() -> AstSearchHit {
        AstSearchHit {
            name: "AlphaSymbol".to_string(),
            signature: "fn AlphaSymbol()".to_string(),
            path: "src/lib.rs".to_string(),
            language: "rust".to_string(),
            crate_name: "kernel".to_string(),
            project_name: None,
            root_label: None,
            node_kind: None,
            owner_title: None,
            navigation_target: StudioNavigationTarget {
                path: "src/lib.rs".to_string(),
                category: "symbol".to_string(),
                project_name: None,
                root_label: None,
                line: Some(1),
                line_end: Some(1),
                column: Some(1),
            },
            line_start: 1,
            line_end: 1,
            score: 0.0,
        }
    }

    fn sample_repo_analysis() -> RepositoryAnalysisOutput {
        RepositoryAnalysisOutput {
            modules: vec![ModuleRecord {
                repo_id: "alpha/repo".to_string(),
                module_id: "module:BaseModelica".to_string(),
                qualified_name: "BaseModelica".to_string(),
                path: "src/BaseModelica.jl".to_string(),
            }],
            symbols: vec![SymbolRecord {
                repo_id: "alpha/repo".to_string(),
                symbol_id: "symbol:reexport".to_string(),
                module_id: Some("module:BaseModelica".to_string()),
                name: "reexport".to_string(),
                qualified_name: "BaseModelica.reexport".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/BaseModelica.jl".to_string(),
                line_start: Some(7),
                line_end: Some(9),
                signature: Some("reexport()".to_string()),
                audit_status: Some("verified".to_string()),
                verification_state: Some("verified".to_string()),
                attributes: std::collections::BTreeMap::new(),
            }],
            examples: vec![ExampleRecord {
                repo_id: "alpha/repo".to_string(),
                example_id: "example:reexport".to_string(),
                title: "Reexport example".to_string(),
                path: "examples/reexport.jl".to_string(),
                summary: Some("Shows how to reexport ModelingToolkit".to_string()),
            }],
            ..RepositoryAnalysisOutput::default()
        }
    }

    #[tokio::test]
    async fn compact_pending_corpus_updates_maintenance_status() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy {
                publish_count_threshold: 1,
                row_delta_ratio_threshold: 1.0,
            },
        );

        let hits = vec![sample_hit()];
        service
            .publish_local_symbol_hits("fp-maintenance", &hits)
            .await
            .expect("publish local symbol hits");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = service
                    .coordinator()
                    .status_for(SearchCorpusKind::LocalSymbol);
                if !status.maintenance.compaction_pending
                    && status.maintenance.last_compacted_at.is_some()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("compaction should complete");

        let status_after = service
            .coordinator()
            .status_for(SearchCorpusKind::LocalSymbol);
        assert!(!status_after.maintenance.compaction_pending);
        assert_eq!(status_after.maintenance.publish_count_since_compaction, 0);
        assert!(status_after.maintenance.last_compacted_at.is_some());
        assert_eq!(
            status_after.maintenance.last_compaction_reason.as_deref(),
            Some("publish_threshold")
        );
        assert_eq!(status_after.fragment_count, Some(1));
    }

    #[tokio::test]
    async fn status_with_repo_content_surfaces_ready_repo_tables() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );
        let documents = vec![RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        }];
        service
            .publish_repo_entities_with_revision(
                "alpha/repo",
                &sample_repo_analysis(),
                Some("rev-1"),
            )
            .await
            .expect("publish repo entities");
        service
            .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
            .await
            .expect("publish repo content chunks");

        let status = service
            .status_with_repo_content(&RepoIndexStatusResponse {
                total: 1,
                active: 0,
                queued: 0,
                checking: 0,
                syncing: 0,
                indexing: 0,
                ready: 1,
                unsupported: 0,
                failed: 0,
                target_concurrency: 1,
                max_concurrency: 1,
                current_repo_id: None,
                active_repo_ids: Vec::new(),
                repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
            })
            .await;

        let repo_content = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            .expect("repo content row should exist");
        assert_eq!(repo_content.phase, SearchPlanePhase::Ready);
        assert!(repo_content.row_count.unwrap_or_default() > 0);
        assert!(repo_content.fragment_count.unwrap_or_default() > 0);
        assert!(repo_content.fingerprint.is_some());
        assert!(repo_content.build_finished_at.is_some());
        assert!(repo_content.updated_at.is_some());
        assert!(repo_content.last_error.is_none());
        assert!(repo_content.issues.is_empty());
        assert!(repo_content.issue_summary.is_none());
        assert!(repo_content.status_reason.is_none());

        let repo_entity = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            .expect("repo entity row should exist");
        assert_eq!(repo_entity.phase, SearchPlanePhase::Ready);
        assert!(repo_entity.row_count.unwrap_or_default() > 0);
        assert!(repo_entity.fragment_count.unwrap_or_default() > 0);
        assert!(repo_entity.fingerprint.is_some());
        assert!(repo_entity.build_finished_at.is_some());
        assert!(repo_entity.updated_at.is_some());
        assert!(repo_entity.last_error.is_none());
        assert!(repo_entity.issues.is_empty());
        assert!(repo_entity.issue_summary.is_none());
        assert!(repo_entity.status_reason.is_none());
    }

    #[tokio::test]
    async fn status_with_repo_content_reports_indexing_before_publish() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );

        let status = service
            .status_with_repo_content(&RepoIndexStatusResponse {
                total: 1,
                active: 1,
                queued: 0,
                checking: 0,
                syncing: 0,
                indexing: 1,
                ready: 0,
                unsupported: 0,
                failed: 0,
                target_concurrency: 1,
                max_concurrency: 1,
                current_repo_id: Some("alpha/repo".to_string()),
                active_repo_ids: vec!["alpha/repo".to_string()],
                repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)],
            })
            .await;

        let repo_content = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            .expect("repo content row should exist");
        assert_eq!(repo_content.phase, SearchPlanePhase::Indexing);
        assert!(repo_content.row_count.is_none());
        assert!(repo_content.fragment_count.is_none());
        assert!(repo_content.fingerprint.is_none());
        assert!(repo_content.build_finished_at.is_none());
        assert!(repo_content.updated_at.is_some());
        assert!(repo_content.last_error.is_none());
        assert!(repo_content.issues.is_empty());
        assert!(repo_content.issue_summary.is_none());
        assert_status_reason(
            repo_content,
            SearchCorpusStatusReasonCode::WarmingUp,
            SearchCorpusStatusSeverity::Info,
            SearchCorpusStatusAction::Wait,
            false,
        );

        let repo_entity = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            .expect("repo entity row should exist");
        assert_eq!(repo_entity.phase, SearchPlanePhase::Indexing);
        assert!(repo_entity.row_count.is_none());
        assert!(repo_entity.fragment_count.is_none());
        assert!(repo_entity.fingerprint.is_none());
        assert!(repo_entity.build_finished_at.is_none());
        assert!(repo_entity.updated_at.is_some());
        assert!(repo_entity.last_error.is_none());
        assert!(repo_entity.issues.is_empty());
        assert!(repo_entity.issue_summary.is_none());
        assert_status_reason(
            repo_entity,
            SearchCorpusStatusReasonCode::WarmingUp,
            SearchCorpusStatusSeverity::Info,
            SearchCorpusStatusAction::Wait,
            false,
        );
    }

    #[tokio::test]
    async fn status_with_repo_content_keeps_published_metadata_while_repo_refreshes() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );
        let documents = vec![RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        }];
        service
            .publish_repo_entities_with_revision(
                "alpha/repo",
                &sample_repo_analysis(),
                Some("rev-0"),
            )
            .await
            .expect("publish repo entities");
        service
            .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-0"))
            .await
            .expect("publish repo content chunks");

        let status = service
            .status_with_repo_content(&RepoIndexStatusResponse {
                total: 1,
                active: 1,
                queued: 0,
                checking: 0,
                syncing: 0,
                indexing: 1,
                ready: 0,
                unsupported: 0,
                failed: 0,
                target_concurrency: 1,
                max_concurrency: 1,
                current_repo_id: Some("alpha/repo".to_string()),
                active_repo_ids: vec!["alpha/repo".to_string()],
                repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)],
            })
            .await;

        let repo_content = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            .expect("repo content row should exist");
        assert_eq!(repo_content.phase, SearchPlanePhase::Indexing);
        assert!(repo_content.row_count.unwrap_or_default() > 0);
        assert!(repo_content.fragment_count.unwrap_or_default() > 0);
        assert!(repo_content.fingerprint.is_some());
        assert!(repo_content.build_finished_at.is_some());
        assert!(repo_content.issues.is_empty());
        assert!(repo_content.issue_summary.is_none());
        assert_status_reason(
            repo_content,
            SearchCorpusStatusReasonCode::Refreshing,
            SearchCorpusStatusSeverity::Info,
            SearchCorpusStatusAction::Wait,
            true,
        );

        let repo_entity = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            .expect("repo entity row should exist");
        assert_eq!(repo_entity.phase, SearchPlanePhase::Indexing);
        assert!(repo_entity.row_count.unwrap_or_default() > 0);
        assert!(repo_entity.fragment_count.unwrap_or_default() > 0);
        assert!(repo_entity.fingerprint.is_some());
        assert!(repo_entity.build_finished_at.is_some());
        assert!(repo_entity.last_error.is_none());
        assert!(repo_entity.issues.is_empty());
        assert!(repo_entity.issue_summary.is_none());
        assert_status_reason(
            repo_entity,
            SearchCorpusStatusReasonCode::Refreshing,
            SearchCorpusStatusSeverity::Info,
            SearchCorpusStatusAction::Wait,
            true,
        );
    }

    #[tokio::test]
    async fn status_with_repo_content_reports_revision_mismatch_for_ready_repo() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );
        let documents = vec![RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        }];
        service
            .publish_repo_entities_with_revision(
                "alpha/repo",
                &sample_repo_analysis(),
                Some("rev-0"),
            )
            .await
            .expect("publish repo entities");
        service
            .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-0"))
            .await
            .expect("publish repo content chunks");

        let status = service
            .status_with_repo_content(&RepoIndexStatusResponse {
                total: 1,
                active: 0,
                queued: 0,
                checking: 0,
                syncing: 0,
                indexing: 0,
                ready: 1,
                unsupported: 0,
                failed: 0,
                target_concurrency: 1,
                max_concurrency: 1,
                current_repo_id: None,
                active_repo_ids: Vec::new(),
                repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
            })
            .await;

        let repo_content = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            .expect("repo content row should exist");
        assert_eq!(repo_content.phase, SearchPlanePhase::Degraded);
        assert!(
            repo_content
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("targets revision `rev-0`"))
        );
        assert_eq!(repo_content.issues.len(), 1);
        assert_eq!(
            repo_content.issues[0].code,
            SearchCorpusIssueCode::PublishedRevisionMismatch
        );
        assert_eq!(
            repo_content.issues[0].repo_id.as_deref(),
            Some("alpha/repo")
        );
        assert_eq!(
            repo_content.issues[0].current_revision.as_deref(),
            Some("rev-1")
        );
        assert_eq!(
            repo_content.issues[0].published_revision.as_deref(),
            Some("rev-0")
        );
        assert!(repo_content.issues[0].readable);
        let repo_content_summary = repo_content
            .issue_summary
            .as_ref()
            .expect("issue summary should be present");
        assert_eq!(
            repo_content_summary.family,
            SearchCorpusIssueFamily::Revision
        );
        assert_eq!(
            repo_content_summary.primary_code,
            SearchCorpusIssueCode::PublishedRevisionMismatch
        );
        assert_eq!(repo_content_summary.issue_count, 1);
        assert_eq!(repo_content_summary.readable_issue_count, 1);
        assert_status_reason(
            repo_content,
            SearchCorpusStatusReasonCode::PublishedRevisionMismatch,
            SearchCorpusStatusSeverity::Warning,
            SearchCorpusStatusAction::ResyncRepo,
            true,
        );

        let repo_entity = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            .expect("repo entity row should exist");
        assert_eq!(repo_entity.phase, SearchPlanePhase::Degraded);
        assert!(
            repo_entity
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("targets revision `rev-0`"))
        );
        assert_eq!(repo_entity.issues.len(), 1);
        assert_eq!(
            repo_entity.issues[0].code,
            SearchCorpusIssueCode::PublishedRevisionMismatch
        );
        assert_eq!(repo_entity.issues[0].repo_id.as_deref(), Some("alpha/repo"));
        assert_eq!(
            repo_entity.issues[0].current_revision.as_deref(),
            Some("rev-1")
        );
        assert_eq!(
            repo_entity.issues[0].published_revision.as_deref(),
            Some("rev-0")
        );
        assert!(repo_entity.issues[0].readable);
        let repo_entity_summary = repo_entity
            .issue_summary
            .as_ref()
            .expect("issue summary should be present");
        assert_eq!(
            repo_entity_summary.family,
            SearchCorpusIssueFamily::Revision
        );
        assert_eq!(
            repo_entity_summary.primary_code,
            SearchCorpusIssueCode::PublishedRevisionMismatch
        );
        assert_eq!(repo_entity_summary.issue_count, 1);
        assert_eq!(repo_entity_summary.readable_issue_count, 1);
        assert_status_reason(
            repo_entity,
            SearchCorpusStatusReasonCode::PublishedRevisionMismatch,
            SearchCorpusStatusSeverity::Warning,
            SearchCorpusStatusAction::ResyncRepo,
            true,
        );
    }

    #[tokio::test]
    async fn status_with_repo_content_requires_repo_manifest_even_when_disk_tables_exist() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );
        let documents = vec![RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        }];
        service
            .publish_repo_entities_with_revision(
                "alpha/repo",
                &sample_repo_analysis(),
                Some("rev-1"),
            )
            .await
            .expect("publish repo entities");
        service
            .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
            .await
            .expect("publish repo content chunks");
        service.clear_repo_publications("alpha/repo");

        assert!(
            !service
                .has_published_repo_corpus(SearchCorpusKind::RepoEntity, "alpha/repo")
                .await
        );
        assert!(
            !service
                .has_published_repo_corpus(SearchCorpusKind::RepoContentChunk, "alpha/repo")
                .await
        );

        let status = service
            .status_with_repo_content(&RepoIndexStatusResponse {
                total: 1,
                active: 0,
                queued: 0,
                checking: 0,
                syncing: 0,
                indexing: 0,
                ready: 1,
                unsupported: 0,
                failed: 0,
                target_concurrency: 1,
                max_concurrency: 1,
                current_repo_id: None,
                active_repo_ids: Vec::new(),
                repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
            })
            .await;

        let repo_content = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            .expect("repo content row should exist");
        assert_eq!(repo_content.phase, SearchPlanePhase::Failed);
        assert!(repo_content.row_count.is_none());
        assert!(repo_content.fragment_count.is_none());
        assert!(repo_content.fingerprint.is_none());
        assert!(
            repo_content
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("published manifest"))
        );
        assert_eq!(repo_content.issues.len(), 1);
        assert_eq!(
            repo_content.issues[0].code,
            SearchCorpusIssueCode::PublishedManifestMissing
        );
        assert_eq!(
            repo_content.issues[0].repo_id.as_deref(),
            Some("alpha/repo")
        );
        assert_eq!(
            repo_content.issues[0].current_revision.as_deref(),
            Some("rev-1")
        );
        assert_eq!(repo_content.issues[0].published_revision, None);
        assert!(!repo_content.issues[0].readable);
        let repo_content_summary = repo_content
            .issue_summary
            .as_ref()
            .expect("issue summary should be present");
        assert_eq!(
            repo_content_summary.family,
            SearchCorpusIssueFamily::Manifest
        );
        assert_eq!(
            repo_content_summary.primary_code,
            SearchCorpusIssueCode::PublishedManifestMissing
        );
        assert_eq!(repo_content_summary.issue_count, 1);
        assert_eq!(repo_content_summary.readable_issue_count, 0);
        assert_status_reason(
            repo_content,
            SearchCorpusStatusReasonCode::PublishedManifestMissing,
            SearchCorpusStatusSeverity::Error,
            SearchCorpusStatusAction::ResyncRepo,
            false,
        );

        let repo_entity = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            .expect("repo entity row should exist");
        assert_eq!(repo_entity.phase, SearchPlanePhase::Failed);
        assert!(repo_entity.row_count.is_none());
        assert!(repo_entity.fragment_count.is_none());
        assert!(repo_entity.fingerprint.is_none());
        assert!(
            repo_entity
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("published manifest"))
        );
        assert_eq!(repo_entity.issues.len(), 1);
        assert_eq!(
            repo_entity.issues[0].code,
            SearchCorpusIssueCode::PublishedManifestMissing
        );
        assert_eq!(repo_entity.issues[0].repo_id.as_deref(), Some("alpha/repo"));
        assert_eq!(
            repo_entity.issues[0].current_revision.as_deref(),
            Some("rev-1")
        );
        assert_eq!(repo_entity.issues[0].published_revision, None);
        assert!(!repo_entity.issues[0].readable);
        let repo_entity_summary = repo_entity
            .issue_summary
            .as_ref()
            .expect("issue summary should be present");
        assert_eq!(
            repo_entity_summary.family,
            SearchCorpusIssueFamily::Manifest
        );
        assert_eq!(
            repo_entity_summary.primary_code,
            SearchCorpusIssueCode::PublishedManifestMissing
        );
        assert_eq!(repo_entity_summary.issue_count, 1);
        assert_eq!(repo_entity_summary.readable_issue_count, 0);
        assert_status_reason(
            repo_entity,
            SearchCorpusStatusReasonCode::PublishedManifestMissing,
            SearchCorpusStatusSeverity::Error,
            SearchCorpusStatusAction::ResyncRepo,
            false,
        );
    }

    #[tokio::test]
    async fn status_with_repo_content_reports_repo_failure_issue_while_rows_remain_readable() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );
        let documents = vec![RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\nlet beta = alpha();\n"),
        }];
        service
            .publish_repo_entities_with_revision(
                "alpha/repo",
                &sample_repo_analysis(),
                Some("rev-1"),
            )
            .await
            .expect("publish repo entities");
        service
            .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
            .await
            .expect("publish repo content chunks");

        let status = service
            .status_with_repo_content(&RepoIndexStatusResponse {
                total: 1,
                active: 0,
                queued: 0,
                checking: 0,
                syncing: 0,
                indexing: 0,
                ready: 0,
                unsupported: 0,
                failed: 1,
                target_concurrency: 1,
                max_concurrency: 1,
                current_repo_id: None,
                active_repo_ids: Vec::new(),
                repos: vec![RepoIndexEntryStatus {
                    last_error: Some("git fetch failed".to_string()),
                    ..repo_status_entry("alpha/repo", RepoIndexPhase::Failed)
                }],
            })
            .await;

        let repo_content = status
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            .expect("repo content row should exist");
        assert_eq!(repo_content.phase, SearchPlanePhase::Degraded);
        assert_eq!(repo_content.issues.len(), 1);
        assert_eq!(
            repo_content.issues[0].code,
            SearchCorpusIssueCode::RepoIndexFailed
        );
        assert!(repo_content.issues[0].readable);
        assert_eq!(
            repo_content.issues[0].published_revision.as_deref(),
            Some("rev-1")
        );
        let repo_content_summary = repo_content
            .issue_summary
            .as_ref()
            .expect("issue summary should be present");
        assert_eq!(
            repo_content_summary.family,
            SearchCorpusIssueFamily::RepoSync
        );
        assert_eq!(
            repo_content_summary.primary_code,
            SearchCorpusIssueCode::RepoIndexFailed
        );
        assert_eq!(repo_content_summary.issue_count, 1);
        assert_eq!(repo_content_summary.readable_issue_count, 1);
        assert_status_reason(
            repo_content,
            SearchCorpusStatusReasonCode::RepoIndexFailed,
            SearchCorpusStatusSeverity::Warning,
            SearchCorpusStatusAction::InspectRepoSync,
            true,
        );
        assert!(
            repo_content
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("git fetch failed"))
        );
    }

    #[tokio::test]
    async fn search_repo_entities_reads_hits_from_published_table() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy::default(),
        );

        service
            .publish_repo_entities_with_revision("alpha/repo", &sample_repo_analysis(), None)
            .await
            .expect("publish repo entities");

        let kind_filters = HashSet::from_iter([String::from("function")]);
        let hits = service
            .search_repo_entities("alpha/repo", "reexport", &HashSet::new(), &kind_filters, 5)
            .await
            .expect("query repo entities");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].doc_type.as_deref(), Some("symbol"));
        assert_eq!(hits[0].stem, "reexport");
        assert_eq!(hits[0].path, "src/BaseModelica.jl");
        assert_eq!(hits[0].match_reason.as_deref(), Some("repo_symbol_search"));
    }

    fn repo_status_entry(repo_id: &str, phase: RepoIndexPhase) -> RepoIndexEntryStatus {
        RepoIndexEntryStatus {
            repo_id: repo_id.to_string(),
            phase,
            queue_position: None,
            last_error: None,
            last_revision: Some("rev-1".to_string()),
            updated_at: Some("2026-03-22T12:00:00Z".to_string()),
            attempt_count: 1,
        }
    }

    fn assert_status_reason(
        status: &SearchCorpusStatus,
        code: SearchCorpusStatusReasonCode,
        severity: SearchCorpusStatusSeverity,
        action: SearchCorpusStatusAction,
        readable: bool,
    ) {
        let reason = status
            .status_reason
            .as_ref()
            .expect("status reason should be present");
        assert_eq!(reason.code, code);
        assert_eq!(reason.severity, severity);
        assert_eq!(reason.action, action);
        assert_eq!(reason.readable, readable);
    }

    #[test]
    fn derive_status_reason_marks_failed_refresh_as_retryable_warning() {
        let mut status = SearchCorpusStatus::new(SearchCorpusKind::LocalSymbol);
        status.phase = SearchPlanePhase::Failed;
        status.active_epoch = Some(7);
        status.row_count = Some(12);
        status.last_error = Some("builder crashed".to_string());

        let reason = derive_status_reason(&status).expect("status reason should exist");

        assert_eq!(reason.code, SearchCorpusStatusReasonCode::BuildFailed);
        assert_eq!(reason.severity, SearchCorpusStatusSeverity::Warning);
        assert_eq!(reason.action, SearchCorpusStatusAction::RetryBuild);
        assert!(reason.readable);
    }

    #[test]
    fn status_marks_ready_corpus_with_pending_compaction_reason() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:search_plane"),
            SearchMaintenancePolicy {
                publish_count_threshold: 1,
                row_delta_ratio_threshold: 1.0,
            },
        );
        let lease = match service.coordinator().begin_build(
            SearchCorpusKind::LocalSymbol,
            "fp-compaction-pending",
            SearchCorpusKind::LocalSymbol.schema_version(),
        ) {
            crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin result: {other:?}"),
        };

        assert!(service.publish_ready_and_maintain(&lease, 10, 3));

        let status = service
            .status()
            .corpora
            .into_iter()
            .find(|entry| entry.corpus == SearchCorpusKind::LocalSymbol)
            .expect("local symbol status should exist");
        assert_eq!(status.phase, SearchPlanePhase::Ready);
        assert!(status.maintenance.compaction_pending);
        assert_status_reason(
            &status,
            SearchCorpusStatusReasonCode::CompactionPending,
            SearchCorpusStatusSeverity::Info,
            SearchCorpusStatusAction::Wait,
            true,
        );
    }

    #[test]
    fn summarize_issues_prefers_highest_priority_code_and_marks_mixed_family() {
        let summary = summarize_issues(&[
            SearchCorpusIssue {
                code: SearchCorpusIssueCode::RepoIndexFailed,
                readable: true,
                repo_id: Some("alpha/repo".to_string()),
                current_revision: Some("rev-2".to_string()),
                published_revision: Some("rev-1".to_string()),
                message: "alpha/repo: git fetch failed".to_string(),
            },
            SearchCorpusIssue {
                code: SearchCorpusIssueCode::PublishedManifestMissing,
                readable: false,
                repo_id: Some("beta/repo".to_string()),
                current_revision: Some("rev-9".to_string()),
                published_revision: None,
                message: "beta/repo: published manifest missing".to_string(),
            },
        ])
        .expect("summary should exist");

        assert_eq!(summary.family, SearchCorpusIssueFamily::Mixed);
        assert_eq!(
            summary.primary_code,
            SearchCorpusIssueCode::PublishedManifestMissing
        );
        assert_eq!(summary.issue_count, 2);
        assert_eq!(summary.readable_issue_count, 1);
    }

    #[test]
    fn repo_corpus_cache_version_tracks_repo_phase_revision_and_timestamp() {
        let ready = repo_corpus_cache_version(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            Some(&repo_status_entry("alpha/repo", RepoIndexPhase::Ready)),
        );
        let indexing = repo_corpus_cache_version(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            Some(&repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)),
        );
        let missing = repo_corpus_cache_version(SearchCorpusKind::RepoEntity, "alpha/repo", None);

        assert_ne!(ready, indexing);
        assert_ne!(ready, missing);
        assert!(ready.contains("phase:ready"));
        assert!(indexing.contains("phase:indexing"));
        assert!(missing.contains("phase:missing"));
    }

    #[test]
    fn repo_publication_cache_version_tracks_refresh_state_without_losing_publication_identity() {
        let publication = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            "repo_entity_repo_alpha",
            1,
            Some("rev-1"),
            7,
            10,
            2,
            "2026-03-23T12:00:00Z",
        );
        let ready_same_revision = repo_status_entry("alpha/repo", RepoIndexPhase::Ready);
        let indexing_new_revision = RepoIndexEntryStatus {
            last_revision: Some("rev-2".to_string()),
            ..repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)
        };

        let ready_key = repo_publication_cache_version(Some(&ready_same_revision), &publication);
        let refreshing_key =
            repo_publication_cache_version(Some(&indexing_new_revision), &publication);

        assert_eq!(ready_key, publication.cache_version());
        assert_ne!(refreshing_key, publication.cache_version());
        assert!(refreshing_key.contains("phase:indexing"));
        assert!(refreshing_key.contains("current-revision:rev-2"));
        assert!(refreshing_key.contains("published-revision:rev-1"));
    }
}
