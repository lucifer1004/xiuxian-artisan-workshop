use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use xiuxian_zhenfa::ZhenfaSignal;

use crate::analyzers::registry::PluginRegistry;
use crate::gateway::studio::pathing;
use crate::gateway::studio::repo_index::RepoIndexCoordinator;
use crate::gateway::studio::symbol_index::{SymbolIndexCoordinator, SymbolIndexStatus};
use crate::gateway::studio::types::{
    AstSearchHit, AttachmentSearchHit, AutocompleteSuggestion, ReferenceSearchHit, SearchHit,
    SearchIndexStatusResponse, UiConfig, UiProjectConfig, VfsScanResult,
};
use crate::link_graph::LinkGraphIndex;
use crate::search_plane::SearchPlaneService;
use crate::unified_symbol::UnifiedSymbolIndex;

use super::config::resolve_studio_config_root;
use super::error::StudioApiError;
use super::repository::configured_repositories;
use super::sanitization::{sanitize_projects, sanitize_repo_projects};

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: std::path::PathBuf,
    pub(crate) config_root: std::path::PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: Arc<RwLock<Option<Arc<LinkGraphIndex>>>>,
    pub(crate) symbol_index: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    pub(crate) symbol_index_coordinator: Arc<SymbolIndexCoordinator>,
    pub(crate) search_plane: SearchPlaneService,
    pub(crate) vfs_scan: Arc<RwLock<Option<VfsScanResult>>>,
    pub(crate) repo_index: Arc<RepoIndexCoordinator>,
    /// Registry of repository intelligence plugins.
    pub(crate) plugin_registry: Arc<PluginRegistry>,
}

/// Shared state used by the top-level gateway process.
#[derive(Clone)]
pub struct GatewayState {
    /// Optional graph index for CLI-powered stats endpoint.
    pub index: Option<Arc<LinkGraphIndex>>,
    /// Signal sender for notification worker.
    pub signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
    /// Studio-specific state for VFS/graph/search APIs.
    pub studio: Arc<StudioState>,
}

impl GatewayState {
    /// Create gateway state shared by the CLI endpoints and Studio router.
    #[must_use]
    pub fn new(
        index: Option<Arc<LinkGraphIndex>>,
        signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
        plugin_registry: Arc<PluginRegistry>,
    ) -> Self {
        Self {
            index,
            signal_tx,
            studio: Arc::new(StudioState::new_with_bootstrap_ui_config(plugin_registry)),
        }
    }

    pub(crate) async fn link_graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        self.studio.graph_index().await
    }
}

impl StudioState {
    /// Create a new `StudioState` with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::new_with_bootstrap_ui_config(Arc::new(PluginRegistry::new()))
    }

    /// Create a new `StudioState` and bootstrap UI config from `wendao.toml`.
    #[must_use]
    pub fn new_with_bootstrap_ui_config(plugin_registry: Arc<PluginRegistry>) -> Self {
        let project_root = xiuxian_io::PrjDirs::project_root();
        let config_root = resolve_studio_config_root(project_root.as_path());
        let search_plane = SearchPlaneService::new(project_root.clone());
        let repo_index = Arc::new(RepoIndexCoordinator::new(
            project_root.clone(),
            Arc::clone(&plugin_registry),
            search_plane.clone(),
        ));
        let symbol_index_coordinator = Arc::new(SymbolIndexCoordinator::new(
            project_root.clone(),
            config_root.clone(),
        ));
        let state = Self {
            project_root,
            config_root,
            ui_config: Arc::new(RwLock::new(UiConfig {
                projects: Vec::new(),
                repo_projects: Vec::new(),
            })),
            graph_index: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            symbol_index_coordinator,
            search_plane,
            vfs_scan: Arc::new(RwLock::new(None)),
            repo_index,
            plugin_registry,
        };
        state.repo_index.start();
        if let Some(config) =
            super::config::load_ui_config_from_wendao_toml(state.config_root.as_path())
        {
            state.set_ui_config(config);
        }
        state
    }

    pub(crate) fn ui_config(&self) -> UiConfig {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn set_ui_config(&self, config: UiConfig) {
        let sanitized_projects = sanitize_projects(config.projects);
        let sanitized_repo_projects = sanitize_repo_projects(config.repo_projects);
        let mut guard = self
            .ui_config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.projects == sanitized_projects && guard.repo_projects == sanitized_repo_projects {
            return;
        }
        guard.projects = sanitized_projects;
        guard.repo_projects = sanitized_repo_projects;
        drop(guard);

        let mut graph_guard = self
            .graph_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *graph_guard = None;
        drop(graph_guard);

        let mut symbol_guard = self
            .symbol_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *symbol_guard = None;
        drop(symbol_guard);

        let mut vfs_guard = self
            .vfs_scan
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *vfs_guard = None;
        drop(vfs_guard);

        self.symbol_index_coordinator
            .sync_projects(self.configured_projects(), Arc::clone(&self.symbol_index));
        self.repo_index
            .sync_repositories(configured_repositories(self));
    }

    pub(crate) fn set_ui_config_and_persist(&self, config: UiConfig) -> Result<(), String> {
        self.set_ui_config(config);
        super::config::persist_ui_config_to_wendao_toml(
            self.config_root.as_path(),
            &self.ui_config(),
        )
    }

    pub(crate) fn configured_projects(
        &self,
    ) -> Vec<crate::gateway::studio::types::UiProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .projects
            .clone()
    }

    pub(crate) fn configured_repo_projects(
        &self,
    ) -> Vec<crate::gateway::studio::types::UiRepoProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_projects
            .clone()
    }

    pub(crate) async fn graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        let project_root = self.project_root.clone();
        let config_root = self.config_root.clone();
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio graph access requires configured link_graph.projects",
            ));
        }

        let build = tokio::task::spawn_blocking(move || {
            let include_dirs = graph_include_dirs(
                project_root.as_path(),
                config_root.as_path(),
                &configured_projects,
            );
            if include_dirs.is_empty() {
                Err(
                    "configured link_graph.projects did not produce any graph include dirs"
                        .to_string(),
                )
            } else {
                LinkGraphIndex::build_with_cache_with_meta(
                    project_root.as_path(),
                    &include_dirs,
                    &[],
                )
                .map(|(index, _meta)| index)
                .or_else(|_| {
                    LinkGraphIndex::build_with_filters(project_root.as_path(), &include_dirs, &[])
                })
            }
        })
        .await
        .map_err(|error: tokio::task::JoinError| {
            StudioApiError::internal(
                "LINK_GRAPH_BUILD_PANIC",
                "Failed to build link graph index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error: String| {
            StudioApiError::internal(
                "LINK_GRAPH_BUILD_FAILED",
                "Failed to build link graph index",
                Some(error),
            )
        })?);
        Ok(index)
    }

    pub(crate) fn current_symbol_index(&self) -> Option<Arc<UnifiedSymbolIndex>> {
        self.symbol_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(Arc::clone)
    }

    pub(crate) fn symbol_index_status(&self) -> Result<SymbolIndexStatus, StudioApiError> {
        let configured_projects = self.configured_projects();

        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio symbol search requires configured link_graph.projects",
            ));
        }

        self.symbol_index_coordinator
            .ensure_started(configured_projects, Arc::clone(&self.symbol_index));
        Ok(self.symbol_index_coordinator.status())
    }

    pub(crate) async fn search_index_status(&self) -> SearchIndexStatusResponse {
        let repo_status = self.repo_index.status_response(None);
        let snapshot = self
            .search_plane
            .status_with_repo_content(&repo_status)
            .await;
        SearchIndexStatusResponse::from(&snapshot)
    }

    pub(crate) fn ensure_local_symbol_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio AST search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_local_symbol_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) fn ensure_knowledge_section_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio knowledge search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_knowledge_section_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn search_knowledge_sections(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, StudioApiError> {
        match self
            .search_plane
            .search_knowledge_sections(query, limit)
            .await
        {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::KnowledgeSectionSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("knowledge_section"))
            }
            Err(error) => Err(StudioApiError::internal(
                "KNOWLEDGE_SECTION_SEARCH_FAILED",
                "Failed to query knowledge section search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) async fn search_local_symbol_hits(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<AstSearchHit>, StudioApiError> {
        match self.search_plane.search_local_symbols(query, limit).await {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::LocalSymbolSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("local_symbol"))
            }
            Err(error) => Err(StudioApiError::internal(
                "LOCAL_SYMBOL_SEARCH_FAILED",
                "Failed to query local symbol search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) async fn autocomplete_local_symbols(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<AutocompleteSuggestion>, StudioApiError> {
        match self
            .search_plane
            .autocomplete_local_symbols(prefix, limit)
            .await
        {
            Ok(suggestions) => Ok(suggestions),
            Err(crate::search_plane::LocalSymbolSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("local_symbol"))
            }
            Err(error) => Err(StudioApiError::internal(
                "LOCAL_SYMBOL_AUTOCOMPLETE_FAILED",
                "Failed to query local symbol autocomplete search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) fn ensure_attachment_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio attachment search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_attachment_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn search_attachment_hits(
        &self,
        query: &str,
        limit: usize,
        extensions: &[String],
        kinds: &[crate::link_graph::LinkGraphAttachmentKind],
        case_sensitive: bool,
    ) -> Result<Vec<AttachmentSearchHit>, StudioApiError> {
        match self
            .search_plane
            .search_attachment_hits(query, limit, extensions, kinds, case_sensitive)
            .await
        {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::AttachmentSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("attachment"))
            }
            Err(error) => Err(StudioApiError::internal(
                "ATTACHMENT_SEARCH_FAILED",
                "Failed to query attachment search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) fn ensure_reference_occurrence_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio reference search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_reference_occurrence_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn search_reference_occurrences(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ReferenceSearchHit>, StudioApiError> {
        match self
            .search_plane
            .search_reference_occurrences(query, limit)
            .await
        {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::ReferenceOccurrenceSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("reference_occurrence"))
            }
            Err(error) => Err(StudioApiError::internal(
                "REFERENCE_OCCURRENCE_SEARCH_FAILED",
                "Failed to query reference occurrence search plane",
                Some(error.to_string()),
            )),
        }
    }
}

impl Default for StudioState {
    fn default() -> Self {
        Self::new()
    }
}

fn graph_include_dirs(
    project_root: &std::path::Path,
    config_root: &std::path::Path,
    projects: &[UiProjectConfig],
) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut include_dirs = Vec::new();

    for project in projects {
        let Some(project_base) = pathing::resolve_path_like(config_root, project.root.as_str())
        else {
            continue;
        };
        for dir_entry in &project.dirs {
            let Some(dir) = pathing::normalize_project_dir_root(dir_entry.as_str()) else {
                continue;
            };
            let Some(candidate) = pathing::resolve_path_like(project_base.as_path(), dir.as_str())
            else {
                continue;
            };
            let Ok(relative) = candidate.strip_prefix(project_root) else {
                continue;
            };
            let normalized = relative
                .to_string_lossy()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_string();
            let value = if normalized.is_empty() {
                ".".to_string()
            } else {
                normalized
            };
            if seen.insert(value.clone()) {
                include_dirs.push(value);
            }
        }
    }

    include_dirs
}
