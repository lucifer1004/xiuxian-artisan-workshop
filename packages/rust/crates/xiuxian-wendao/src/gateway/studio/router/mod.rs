//! Studio API router for Qianji frontend.
//!
//! Provides HTTP endpoints for VFS operations, graph queries, and UI configuration.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use tokio::sync::Mutex;
use xiuxian_zhenfa::ZhenfaSignal;

use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepositoryPluginConfig, RepositoryRef,
    RepositoryRefreshPolicy,
};
use crate::gateway::openapi::paths as openapi_paths;
use crate::gateway::studio::pathing;
use crate::gateway::studio::repo_index::RepoIndexCoordinator;
use crate::gateway::studio::search;
use crate::gateway::studio::types::{
    ApiError, UiConfig, UiProjectConfig, UiRepoProjectConfig, VfsScanResult,
};
use crate::link_graph::LinkGraphIndex;
use crate::unified_symbol::UnifiedSymbolIndex;

/// Code-AST response builders and repository/path resolution helpers.
pub mod code_ast;
pub mod config;
pub mod handlers;
pub mod sanitization;

pub use code_ast::build_code_ast_analysis_response;
pub use config::{
    load_ui_config_from_wendao_toml, persist_ui_config_to_wendao_toml, resolve_studio_config_root,
    studio_wendao_toml_path,
};
pub use sanitization::{
    sanitize_path_like, sanitize_path_list, sanitize_projects, sanitize_repo_projects,
};

// Re-export handlers for convenience
pub use handlers::{
    code_ast, doc_coverage, example_search, get_ui_config, graph_neighbors, markdown,
    module_search, node_neighbors, overview, projected_page, projected_page_family_cluster,
    projected_page_family_context, projected_page_family_search, projected_page_index_node,
    projected_page_index_tree, projected_page_index_tree_search, projected_page_index_trees,
    projected_page_navigation, projected_page_navigation_search, projected_page_search,
    projected_pages, projected_retrieval, projected_retrieval_context, projected_retrieval_hit,
    refine_entity_doc, set_ui_config, symbol_search, sync, topology_3d, vfs_cat, vfs_entry,
    vfs_resolve, vfs_root_entries, vfs_scan,
};

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: std::path::PathBuf,
    pub(crate) config_root: std::path::PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: Arc<RwLock<Option<Arc<LinkGraphIndex>>>>,
    pub(crate) symbol_index: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    pub(crate) symbol_index_build_lock: Arc<Mutex<()>>,
    pub(crate) ast_index:
        Arc<RwLock<Option<Arc<Vec<crate::gateway::studio::types::AstSearchHit>>>>>,
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
        let repo_index = Arc::new(RepoIndexCoordinator::new(
            project_root.clone(),
            Arc::clone(&plugin_registry),
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
            symbol_index_build_lock: Arc::new(Mutex::new(())),
            ast_index: Arc::new(RwLock::new(None)),
            vfs_scan: Arc::new(RwLock::new(None)),
            repo_index,
            plugin_registry,
        };
        state.repo_index.start();
        if let Some(config) = load_ui_config_from_wendao_toml(state.config_root.as_path()) {
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

        let mut ast_guard = self
            .ast_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *ast_guard = None;

        let mut vfs_guard = self
            .vfs_scan
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *vfs_guard = None;
        drop(vfs_guard);

        self.repo_index
            .sync_repositories(configured_repositories(self));
    }

    pub(crate) fn set_ui_config_and_persist(&self, config: UiConfig) -> Result<(), String> {
        self.set_ui_config(config);
        persist_ui_config_to_wendao_toml(self.config_root.as_path(), &self.ui_config())
    }

    pub(crate) fn configured_projects(&self) -> Vec<UiProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .projects
            .clone()
    }

    pub(crate) fn configured_repo_projects(&self) -> Vec<UiRepoProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_projects
            .clone()
    }

    pub(crate) async fn graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        if let Some(index) = self
            .graph_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

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
                LinkGraphIndex::build_with_filters(project_root.as_path(), &include_dirs, &[])
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

        let mut guard = self
            .graph_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = guard.as_ref() {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&index));
        Ok(index)
    }

    pub(crate) async fn symbol_index(&self) -> Result<Arc<UnifiedSymbolIndex>, StudioApiError> {
        let project_root = self.project_root.clone();
        let config_root = self.config_root.clone();
        let configured_projects = self.configured_projects();

        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio symbol search requires configured link_graph.projects",
            ));
        }

        if let Some(index) = self
            .symbol_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

        let _build_guard = self.symbol_index_build_lock.lock().await;
        if let Some(index) = self
            .symbol_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

        let build = tokio::task::spawn_blocking(move || {
            search::build_symbol_index(
                project_root.as_path(),
                config_root.as_path(),
                &configured_projects,
            )
        })
        .await
        .map_err(|error: tokio::task::JoinError| {
            StudioApiError::internal(
                "SYMBOL_INDEX_BUILD_PANIC",
                "Failed to build studio symbol index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error: String| {
            StudioApiError::internal(
                "SYMBOL_INDEX_BUILD_FAILED",
                "Failed to build studio symbol index",
                Some(error),
            )
        })?);

        let mut guard = self
            .symbol_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = guard.as_ref() {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&index));
        Ok(index)
    }

    pub(crate) async fn ast_index(
        &self,
    ) -> Result<Arc<Vec<crate::gateway::studio::types::AstSearchHit>>, StudioApiError> {
        let project_root = self.project_root.clone();
        let config_root = self.config_root.clone();
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio AST search requires configured link_graph.projects",
            ));
        }

        if let Some(index) = self
            .ast_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

        let build = tokio::task::spawn_blocking(move || {
            search::build_ast_index(
                project_root.as_path(),
                config_root.as_path(),
                &configured_projects,
            )
        })
        .await
        .map_err(|error: tokio::task::JoinError| {
            StudioApiError::internal(
                "AST_INDEX_BUILD_PANIC",
                "Failed to build studio AST index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error: String| {
            StudioApiError::internal(
                "AST_INDEX_BUILD_FAILED",
                "Failed to build studio AST index",
                Some(error),
            )
        })?);

        let mut guard = self
            .ast_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = guard.as_ref() {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&index));
        Ok(index)
    }
}

impl Default for StudioState {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the configured repository by ID.
pub fn configured_repository(
    studio: &StudioState,
    repo_id: &str,
) -> Result<RegisteredRepository, RepoIntelligenceError> {
    configured_repositories(studio)
        .into_iter()
        .find(|repository| repository.id == repo_id)
        .ok_or_else(|| RepoIntelligenceError::UnknownRepository {
            repo_id: repo_id.to_string(),
        })
}

/// Returns all configured repositories.
pub fn configured_repositories(studio: &StudioState) -> Vec<RegisteredRepository> {
    studio
        .configured_repo_projects()
        .into_iter()
        .filter_map(|project| {
            if project.plugins.is_empty() {
                return None;
            }
            let path = project
                .root
                .as_deref()
                .and_then(|root| pathing::resolve_path_like(studio.config_root.as_path(), root));
            let url = project.url.map(|value| value.trim().to_string());
            if path.is_none() && url.is_none() {
                return None;
            }
            Some(RegisteredRepository {
                id: project.id,
                path,
                url,
                git_ref: project.git_ref.map(RepositoryRef::Branch),
                refresh: parse_refresh_policy(project.refresh.as_deref()),
                plugins: project
                    .plugins
                    .into_iter()
                    .map(RepositoryPluginConfig::Id)
                    .collect(),
            })
        })
        .collect()
}

fn parse_refresh_policy(refresh: Option<&str>) -> RepositoryRefreshPolicy {
    match refresh
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("fetch")
    {
        "manual" => RepositoryRefreshPolicy::Manual,
        _ => RepositoryRefreshPolicy::Fetch,
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

/// Studio API error type.
#[derive(Debug, serde::Serialize, Clone)]
pub struct StudioApiError {
    #[serde(skip)]
    /// HTTP status returned for the error.
    pub status: StatusCode,
    /// Serialized API error payload.
    pub error: ApiError,
}

impl StudioApiError {
    /// Return the HTTP status associated with the error.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Return the stable error code carried by the payload.
    pub fn code(&self) -> &str {
        self.error.code.as_str()
    }

    /// Creates a bad request error.
    pub fn bad_request(code: &str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: ApiError {
                code: code.to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// Creates a not found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: ApiError {
                code: "NOT_FOUND".to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// Creates an internal server error.
    pub fn internal(code: &str, message: impl Into<String>, details: Option<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: ApiError {
                code: code.to_string(),
                message: message.into(),
                details,
            },
        }
    }

    /// Creates a conflict error.
    pub fn conflict(code: &str, message: impl Into<String>, details: Option<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            error: ApiError {
                code: code.to_string(),
                message: message.into(),
                details,
            },
        }
    }
}

impl IntoResponse for StudioApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.error.clone())).into_response()
    }
}

/// Maps a `RepoIntelligenceError` to a `StudioApiError`.
pub fn map_repo_intelligence_error(error: RepoIntelligenceError) -> StudioApiError {
    match error {
        RepoIntelligenceError::UnknownRepository { repo_id } => StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            format!("Repo Intelligence repository `{repo_id}` is not registered"),
        ),
        RepoIntelligenceError::MissingRequiredPlugin { repo_id, plugin_id } => {
            StudioApiError::bad_request(
                "MISSING_REQUIRED_PLUGIN",
                format!("repo `{repo_id}` requires plugin `{plugin_id}`"),
            )
        }
        RepoIntelligenceError::MissingPlugin { plugin_id } => StudioApiError::bad_request(
            "MISSING_PLUGIN",
            format!("repo intelligence plugin `{plugin_id}` is not registered"),
        ),
        RepoIntelligenceError::MissingRepositoryPath { repo_id } => StudioApiError::bad_request(
            "MISSING_REPOSITORY_PATH",
            format!("repo `{repo_id}` does not declare a local path"),
        ),
        RepoIntelligenceError::MissingRepositorySource { repo_id } => StudioApiError::bad_request(
            "MISSING_REPOSITORY_SOURCE",
            format!("repo `{repo_id}` must declare a local path or upstream url"),
        ),
        RepoIntelligenceError::InvalidRepositoryPath { path, reason, .. } => {
            StudioApiError::bad_request(
                "INVALID_REPOSITORY_PATH",
                format!("invalid repository path `{path}`: {reason}"),
            )
        }
        RepoIntelligenceError::UnsupportedRepositoryLayout { repo_id, message } => {
            StudioApiError::bad_request(
                "UNSUPPORTED_REPOSITORY_LAYOUT",
                format!("repo `{repo_id}` has unsupported layout: {message}"),
            )
        }
        RepoIntelligenceError::PendingRepositoryIndex { repo_id } => StudioApiError::conflict(
            "REPO_INDEX_PENDING",
            format!("repo `{repo_id}` index is still warming"),
            Some(repo_id),
        ),
        RepoIntelligenceError::UnknownProjectedPage { repo_id, page_id } => {
            StudioApiError::not_found(format!(
                "repo `{repo_id}` does not contain projected page `{page_id}`"
            ))
        }
        RepoIntelligenceError::UnknownProjectedPageFamilyCluster {
            repo_id,
            page_id,
            kind,
        } => StudioApiError::not_found(format!(
            "repo `{repo_id}` does not contain projected page family `{kind:?}` in page `{page_id}`"
        )),
        RepoIntelligenceError::UnknownProjectedPageIndexNode {
            repo_id,
            page_id,
            node_id,
        } => StudioApiError::not_found(format!(
            "repo `{repo_id}` does not contain projected page-index node `{node_id}` in page `{page_id}`"
        )),
        RepoIntelligenceError::ConfigLoad { message } => {
            StudioApiError::bad_request("CONFIG_LOAD_FAILED", message)
        }
        RepoIntelligenceError::DuplicatePlugin { plugin_id } => StudioApiError::internal(
            "DUPLICATE_PLUGIN",
            "Repo intelligence plugin registry is inconsistent",
            Some(format!("duplicate plugin `{plugin_id}`")),
        ),
        RepoIntelligenceError::AnalysisFailed { message } => StudioApiError::internal(
            "REPO_INTELLIGENCE_FAILED",
            "Repo intelligence task failed",
            Some(message),
        ),
    }
}

/// Create the Studio API router with all endpoints.
pub fn studio_routes() -> Router<Arc<GatewayState>> {
    Router::new()
        .route(
            openapi_paths::API_VFS_ROOT_AXUM_PATH,
            get(handlers::vfs_root_entries),
        )
        .route(
            openapi_paths::API_VFS_SCAN_AXUM_PATH,
            get(handlers::vfs_scan),
        )
        .route(openapi_paths::API_VFS_CAT_AXUM_PATH, get(handlers::vfs_cat))
        .route("/api/vfs/resolve", get(handlers::vfs_resolve))
        .route(
            openapi_paths::API_VFS_ENTRY_AXUM_PATH,
            get(handlers::vfs_entry),
        )
        .route(
            openapi_paths::API_NEIGHBORS_AXUM_PATH,
            get(handlers::node_neighbors),
        )
        .route(
            openapi_paths::API_GRAPH_NEIGHBORS_AXUM_PATH,
            get(handlers::graph_neighbors),
        )
        .route(
            openapi_paths::API_TOPOLOGY_3D_AXUM_PATH,
            get(handlers::topology_3d),
        )
        .route(
            openapi_paths::API_SEARCH_AXUM_PATH,
            get(search::search_knowledge),
        )
        .route(
            openapi_paths::API_SEARCH_INTENT_AXUM_PATH,
            get(search::search_intent),
        )
        .route(
            openapi_paths::API_SEARCH_ATTACHMENTS_AXUM_PATH,
            get(search::search_attachments),
        )
        .route(
            openapi_paths::API_SEARCH_AST_AXUM_PATH,
            get(search::search_ast),
        )
        .route(
            openapi_paths::API_SEARCH_DEFINITION_AXUM_PATH,
            get(search::search_definition),
        )
        .route(
            openapi_paths::API_SEARCH_REFERENCES_AXUM_PATH,
            get(search::search_references),
        )
        .route(
            openapi_paths::API_SEARCH_SYMBOLS_AXUM_PATH,
            get(search::search_symbols),
        )
        .route(
            openapi_paths::API_SEARCH_AUTOCOMPLETE_AXUM_PATH,
            get(search::search_autocomplete),
        )
        .route(
            openapi_paths::API_ANALYSIS_MARKDOWN_AXUM_PATH,
            get(handlers::markdown),
        )
        .route(
            openapi_paths::API_ANALYSIS_CODE_AST_AXUM_PATH,
            get(handlers::code_ast),
        )
        .route(
            openapi_paths::API_UI_CONFIG_AXUM_PATH,
            get(handlers::get_ui_config).post(handlers::set_ui_config),
        )
        .route(
            "/api/analysis/refine-doc",
            post(handlers::refine_entity_doc),
        )
        .route(
            openapi_paths::API_REPO_OVERVIEW_AXUM_PATH,
            get(handlers::overview),
        )
        .route(
            openapi_paths::API_REPO_MODULE_SEARCH_AXUM_PATH,
            get(handlers::module_search),
        )
        .route(
            openapi_paths::API_REPO_SYMBOL_SEARCH_AXUM_PATH,
            get(handlers::symbol_search),
        )
        .route(
            openapi_paths::API_REPO_EXAMPLE_SEARCH_AXUM_PATH,
            get(handlers::example_search),
        )
        .route(
            openapi_paths::API_REPO_DOC_COVERAGE_AXUM_PATH,
            get(handlers::doc_coverage),
        )
        .route(
            openapi_paths::API_REPO_INDEX_STATUS_AXUM_PATH,
            get(handlers::repo_index_status),
        )
        .route(
            openapi_paths::API_REPO_INDEX_AXUM_PATH,
            post(handlers::repo_index),
        )
        .route(openapi_paths::API_REPO_SYNC_AXUM_PATH, get(handlers::sync))
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGES_AXUM_PATH,
            get(handlers::projected_pages),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_AXUM_PATH,
            get(handlers::projected_page),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_NODE_AXUM_PATH,
            get(handlers::repo::projected_page_index_node),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_RETRIEVAL_HIT_AXUM_PATH,
            get(handlers::repo::projected_retrieval_hit),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_RETRIEVAL_CONTEXT_AXUM_PATH,
            get(handlers::repo::projected_retrieval_context),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_AXUM_PATH,
            get(handlers::repo::projected_page_family_context),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_AXUM_PATH,
            get(handlers::repo::projected_page_family_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_AXUM_PATH,
            get(handlers::repo::projected_page_family_cluster),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_NAVIGATION_AXUM_PATH,
            get(handlers::repo::projected_page_navigation),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_AXUM_PATH,
            get(handlers::repo::projected_page_navigation_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_TREE_AXUM_PATH,
            get(handlers::repo::projected_page_index_tree),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_AXUM_PATH,
            get(handlers::repo::projected_page_index_tree_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_SEARCH_AXUM_PATH,
            get(handlers::repo::projected_page_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_RETRIEVAL_AXUM_PATH,
            get(handlers::repo::projected_retrieval),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_TREES_AXUM_PATH,
            get(handlers::repo::projected_page_index_trees),
        )
}

/// Create the Studio API router with state already attached.
pub fn studio_router(state: Arc<GatewayState>) -> Router {
    studio_routes().with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::{ModuleRecord, RelationRecord, RepoSymbolKind, SymbolRecord};
    use std::collections::BTreeMap;

    fn studio_with_repo_projects(repo_projects: Vec<UiRepoProjectConfig>) -> StudioState {
        let studio = StudioState::new();
        studio.set_ui_config(UiConfig {
            projects: Vec::new(),
            repo_projects,
        });
        studio
    }

    fn repo_project(id: &str) -> UiRepoProjectConfig {
        UiRepoProjectConfig {
            id: id.to_string(),
            root: Some(".".to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }
    }

    #[test]
    fn set_ui_config_preserves_cached_state_when_effectively_unchanged() {
        let studio = StudioState::new();
        let config = UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: vec![repo_project("sciml")],
        };
        studio.set_ui_config(config.clone());

        *studio
            .symbol_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Arc::new(UnifiedSymbolIndex::new()));
        *studio
            .vfs_scan
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(VfsScanResult {
            entries: Vec::new(),
            file_count: 0,
            dir_count: 0,
            scan_duration_ms: 0,
        });

        studio.set_ui_config(config);

        assert!(
            studio
                .symbol_index
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_some()
        );
        assert!(
            studio
                .vfs_scan
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_some()
        );
    }

    #[test]
    fn resolve_code_ast_repository_and_path_infers_repo_from_prefixed_path() {
        use code_ast::resolve_code_ast_repository_and_path;
        let studio = studio_with_repo_projects(vec![repo_project("sciml"), repo_project("mcl")]);
        let repositories = configured_repositories(&studio);
        let (repository, path) =
            resolve_code_ast_repository_and_path(&repositories, None, "sciml/src/BaseModelica.jl")
                .expect("repo should be inferred from prefixed path");
        assert_eq!(repository.id, "sciml");
        assert_eq!(path, "src/BaseModelica.jl");
    }

    #[test]
    fn resolve_code_ast_repository_and_path_requires_repo_when_ambiguous() {
        use code_ast::resolve_code_ast_repository_and_path;
        let studio = studio_with_repo_projects(vec![repo_project("sciml"), repo_project("mcl")]);
        let repositories = configured_repositories(&studio);
        let error =
            resolve_code_ast_repository_and_path(&repositories, None, "src/BaseModelica.jl")
                .expect_err("should fail when repo cannot be inferred");
        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error.code(), "MISSING_REPO");
    }

    #[test]
    fn build_code_ast_analysis_response_emits_uses_projection_and_external_node() {
        use crate::gateway::studio::types::{
            CodeAstEdgeKind, CodeAstNodeKind, CodeAstProjectionKind,
        };
        let analysis = crate::analyzers::RepositoryAnalysisOutput {
            modules: vec![ModuleRecord {
                repo_id: "sciml".to_string(),
                module_id: "module:BaseModelica".to_string(),
                qualified_name: "BaseModelica".to_string(),
                path: "src/BaseModelica.jl".to_string(),
            }],
            symbols: vec![
                SymbolRecord {
                    repo_id: "sciml".to_string(),
                    symbol_id: "symbol:reexport".to_string(),
                    module_id: Some("module:BaseModelica".to_string()),
                    name: "reexport".to_string(),
                    qualified_name: "BaseModelica.reexport".to_string(),
                    kind: RepoSymbolKind::Function,
                    path: "src/BaseModelica.jl".to_string(),
                    line_start: Some(7),
                    line_end: Some(9),
                    signature: None,
                    audit_status: None,
                    verification_state: None,
                    attributes: BTreeMap::new(),
                },
                SymbolRecord {
                    repo_id: "sciml".to_string(),
                    symbol_id: "symbol:ModelicaSystem".to_string(),
                    module_id: None,
                    name: "ModelicaSystem".to_string(),
                    qualified_name: "ModelicaSystem".to_string(),
                    kind: RepoSymbolKind::Type,
                    path: "src/modelica/system.jl".to_string(),
                    line_start: Some(1),
                    line_end: Some(3),
                    signature: None,
                    audit_status: None,
                    verification_state: None,
                    attributes: BTreeMap::new(),
                },
            ],
            relations: vec![RelationRecord {
                repo_id: "sciml".to_string(),
                source_id: "symbol:reexport".to_string(),
                target_id: "symbol:ModelicaSystem".to_string(),
                kind: crate::analyzers::RelationKind::Uses,
            }],
            ..crate::analyzers::RepositoryAnalysisOutput::default()
        };
        let payload = build_code_ast_analysis_response(
            "sciml".to_string(),
            "src/BaseModelica.jl".to_string(),
            Some(7),
            &analysis,
        );
        assert_eq!(payload.language, "julia");
        assert!(
            payload
                .nodes
                .iter()
                .any(|node| matches!(node.kind, CodeAstNodeKind::ExternalSymbol))
        );
        assert!(
            payload
                .edges
                .iter()
                .any(|edge| matches!(edge.kind, CodeAstEdgeKind::Uses))
        );
        assert!(payload.projections.iter().any(|projection| {
            matches!(projection.kind, CodeAstProjectionKind::Calls) && projection.edge_count > 0
        }));
        assert!(payload.focus_node_id.is_some());
    }
}
