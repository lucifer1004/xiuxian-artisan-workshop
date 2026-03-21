use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use xiuxian_io::PrjDirs;
use xiuxian_zhenfa::ZhenfaSignal;

use crate::gateway::openapi::paths as openapi_paths;
use crate::link_graph::LinkGraphIndex;
use crate::repo_intelligence::{
    DocCoverageQuery, ExampleSearchQuery, ModuleSearchQuery, ProjectionPageKind,
    RegisteredRepository, RepoIntelligenceError, RepoOverviewQuery,
    RepoProjectedPageFamilyClusterQuery,
    RepoProjectedPageFamilyContextQuery, RepoProjectedPageFamilySearchQuery,
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexTreeQuery,
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageIndexTreesQuery,
    RepoProjectedPageNavigationQuery, RepoProjectedPageNavigationSearchQuery,
    RepoProjectedPageQuery, RepoProjectedPageSearchQuery, RepoProjectedPagesQuery,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalHitQuery,
    RepoProjectedRetrievalQuery, RepoSyncMode, RepoSyncQuery, RepositoryPluginConfig,
    RepositoryRef, RepositoryRefreshPolicy, SymbolSearchQuery, analyze_registered_repository,
    build_doc_coverage, build_example_search, build_module_search, build_repo_overview,
    build_repo_projected_page, build_repo_projected_page_family_cluster,
    build_repo_projected_page_family_context, build_repo_projected_page_family_search,
    build_repo_projected_page_index_node, build_repo_projected_page_index_tree,
    build_repo_projected_page_index_tree_search, build_repo_projected_page_index_trees,
    build_repo_projected_page_navigation, build_repo_projected_page_navigation_search,
    build_repo_projected_page_search, build_repo_projected_pages, build_repo_projected_retrieval,
    build_repo_projected_retrieval_context, build_repo_projected_retrieval_hit,
    build_symbol_search, repo_sync_for_registered_repository,
};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::types::{
    ApiError, AstSearchHit, GraphNeighborsResponse, MarkdownAnalysisResponse, NodeNeighbors,
    UiConfig, UiProjectConfig, UiRepoProjectConfig, VfsContentResponse, VfsEntry, VfsScanResult,
};
use super::{analysis, graph, pathing, search, vfs};

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: PathBuf,
    pub(crate) config_root: PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: Arc<RwLock<Option<Arc<LinkGraphIndex>>>>,
    pub(crate) symbol_index: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    pub(crate) ast_index: Arc<RwLock<Option<Arc<Vec<AstSearchHit>>>>>,
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
    ) -> Self {
        Self {
            index,
            signal_tx,
            studio: Arc::new(StudioState::new()),
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
        let project_root = PrjDirs::project_root();
        let config_root = resolve_studio_config_root(project_root.as_path());
        Self {
            project_root,
            config_root,
            ui_config: Arc::new(RwLock::new(UiConfig {
                projects: Vec::new(),
                repo_projects: Vec::new(),
            })),
            graph_index: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            ast_index: Arc::new(RwLock::new(None)),
        }
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
        .map_err(|error| {
            StudioApiError::internal(
                "LINK_GRAPH_BUILD_PANIC",
                "Failed to build link graph index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error| {
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

        let build = tokio::task::spawn_blocking(move || {
            search::build_symbol_index(
                project_root.as_path(),
                config_root.as_path(),
                &configured_projects,
            )
        })
        .await
        .map_err(|error| {
            StudioApiError::internal(
                "SYMBOL_INDEX_BUILD_PANIC",
                "Failed to build studio symbol index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error| {
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

    pub(crate) async fn ast_index(&self) -> Result<Arc<Vec<AstSearchHit>>, StudioApiError> {
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
        .map_err(|error| {
            StudioApiError::internal(
                "AST_INDEX_BUILD_PANIC",
                "Failed to build studio AST index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error| {
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

#[derive(Debug, Deserialize)]
struct VfsCatQuery {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphNeighborsQuery {
    direction: Option<String>,
    hops: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct MarkdownAnalysisQuery {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoApiQuery {
    repo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoSearchApiQuery {
    repo: Option<String>,
    query: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageIndexNodeApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedRetrievalHitApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedRetrievalContextApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
    node_id: Option<String>,
    related_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageFamilyContextApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
    per_kind_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageFamilySearchApiQuery {
    repo: Option<String>,
    query: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
    per_kind_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageFamilyClusterApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageNavigationApiQuery {
    repo: Option<String>,
    page_id: Option<String>,
    node_id: Option<String>,
    family_kind: Option<String>,
    related_limit: Option<usize>,
    family_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageNavigationSearchApiQuery {
    repo: Option<String>,
    query: Option<String>,
    kind: Option<String>,
    family_kind: Option<String>,
    limit: Option<usize>,
    related_limit: Option<usize>,
    family_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoProjectedPageSearchApiQuery {
    repo: Option<String>,
    query: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RepoDocCoverageApiQuery {
    repo: Option<String>,
    #[serde(rename = "module")]
    module_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoSyncApiQuery {
    repo: Option<String>,
    mode: Option<String>,
}

/// Create the Studio API router with all endpoints.
///
/// # Endpoints
///
/// - `GET /api/vfs` - List root entries
/// - `GET /api/vfs/scan` - Scan all VFS roots
/// - `GET /api/vfs/cat?path=` - Read file content
/// - `GET /api/vfs/resolve?path=` - Resolve a studio navigation target from a semantic path
/// - `GET /api/vfs/{*path}` - Get single entry
/// - `GET /api/neighbors/{*id}` - Get node neighbors
/// - `GET /api/graph/neighbors/{*id}` - Get graph neighbors
/// - `GET /api/topology/3d` - Get deterministic graph topology payload
/// - `GET /api/search` - Search knowledge base
/// - `GET /api/search/intent` - Search knowledge base with explicit intent hints
/// - `GET /api/search/attachments` - Search markdown attachment references
/// - `GET /api/search/ast` - Search AST definitions
/// - `GET /api/search/definition` - Resolve the best semantic definition hit
/// - `GET /api/search/references` - Search symbol references and usages
/// - `GET /api/search/symbols` - Search project symbols
/// - `GET /api/search/autocomplete` - Search autocomplete suggestions
/// - `GET /api/analysis/markdown?path=` - Compile Markdown structural IR + Mermaid projections
/// - `GET/POST /api/ui/config` - UI configuration
/// - `GET /api/repo/overview?repo=` - Repo Intelligence repository overview
/// - `GET /api/repo/module-search?repo=&query=&limit=` - Repo Intelligence module search
/// - `GET /api/repo/symbol-search?repo=&query=&limit=` - Repo Intelligence symbol search
/// - `GET /api/repo/example-search?repo=&query=&limit=` - Repo Intelligence example search
/// - `GET /api/repo/doc-coverage?repo=&module=` - Repo Intelligence doc coverage
/// - `GET /api/repo/sync?repo=&mode=` - Repo Intelligence source synchronization status
/// - `GET /api/repo/projected-pages?repo=` - Repo Intelligence deterministic projected pages
/// - `GET /api/repo/projected-page-index-node?repo=&page_id=&node_id=` - Repo Intelligence deterministic projected page-index node lookup
/// - `GET /api/repo/projected-retrieval-hit?repo=&page_id=&node_id=` - Repo Intelligence deterministic mixed Stage-2 hit lookup
/// - `GET /api/repo/projected-retrieval-context?repo=&page_id=&node_id=&related_limit=` - Repo Intelligence deterministic mixed Stage-2 local context lookup
/// - `GET /api/repo/projected-page-family-context?repo=&page_id=&per_kind_limit=` - Repo Intelligence deterministic projected page-family context lookup
/// - `GET /api/repo/projected-page-family-search?repo=&query=&kind=&limit=&per_kind_limit=` - Repo Intelligence deterministic projected page-family cluster search
/// - `GET /api/repo/projected-page-family-cluster?repo=&page_id=&kind=&limit=` - Repo Intelligence deterministic projected page-family cluster lookup
/// - `GET /api/repo/projected-page-navigation?repo=&page_id=&node_id=&family_kind=&related_limit=&family_limit=` - Repo Intelligence deterministic projected page navigation bundle
/// - `GET /api/repo/projected-page-navigation-search?repo=&query=&kind=&family_kind=&limit=&related_limit=&family_limit=` - Repo Intelligence deterministic projected page navigation search
/// - `GET /api/repo/projected-page-search?repo=&query=&kind=&limit=` - Repo Intelligence deterministic projected page retrieval
/// - `GET /api/repo/projected-retrieval?repo=&query=&kind=&limit=` - Repo Intelligence deterministic mixed Stage-2 retrieval
/// - `GET /api/repo/projected-page-index-trees?repo=` - Repo Intelligence deterministic projected page-index trees
pub fn studio_routes() -> Router<Arc<GatewayState>> {
    Router::new()
        .route(openapi_paths::API_VFS_ROOT_AXUM_PATH, get(vfs_root_entries))
        .route(openapi_paths::API_VFS_SCAN_AXUM_PATH, get(vfs_scan))
        .route(openapi_paths::API_VFS_CAT_AXUM_PATH, get(vfs_cat))
        .route("/api/vfs/resolve", get(vfs_resolve))
        .route(openapi_paths::API_VFS_ENTRY_AXUM_PATH, get(vfs_entry))
        .route(openapi_paths::API_NEIGHBORS_AXUM_PATH, get(node_neighbors))
        .route(
            openapi_paths::API_GRAPH_NEIGHBORS_AXUM_PATH,
            get(graph_neighbors),
        )
        .route(openapi_paths::API_TOPOLOGY_3D_AXUM_PATH, get(topology_3d))
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
            get(analysis_markdown),
        )
        .route(
            openapi_paths::API_UI_CONFIG_AXUM_PATH,
            get(get_ui_config).post(set_ui_config),
        )
        .route(
            openapi_paths::API_REPO_OVERVIEW_AXUM_PATH,
            get(repo_overview),
        )
        .route(
            openapi_paths::API_REPO_MODULE_SEARCH_AXUM_PATH,
            get(repo_module_search),
        )
        .route(
            openapi_paths::API_REPO_SYMBOL_SEARCH_AXUM_PATH,
            get(repo_symbol_search),
        )
        .route(
            openapi_paths::API_REPO_EXAMPLE_SEARCH_AXUM_PATH,
            get(repo_example_search),
        )
        .route(
            openapi_paths::API_REPO_DOC_COVERAGE_AXUM_PATH,
            get(repo_doc_coverage),
        )
        .route(openapi_paths::API_REPO_SYNC_AXUM_PATH, get(repo_sync))
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGES_AXUM_PATH,
            get(repo_projected_pages),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_AXUM_PATH,
            get(repo_projected_page),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_NODE_AXUM_PATH,
            get(repo_projected_page_index_node),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_RETRIEVAL_HIT_AXUM_PATH,
            get(repo_projected_retrieval_hit),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_RETRIEVAL_CONTEXT_AXUM_PATH,
            get(repo_projected_retrieval_context),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_AXUM_PATH,
            get(repo_projected_page_family_context),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_AXUM_PATH,
            get(repo_projected_page_family_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_AXUM_PATH,
            get(repo_projected_page_family_cluster),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_NAVIGATION_AXUM_PATH,
            get(repo_projected_page_navigation),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_AXUM_PATH,
            get(repo_projected_page_navigation_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_TREE_AXUM_PATH,
            get(repo_projected_page_index_tree),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_AXUM_PATH,
            get(repo_projected_page_index_tree_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_SEARCH_AXUM_PATH,
            get(repo_projected_page_search),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_RETRIEVAL_AXUM_PATH,
            get(repo_projected_retrieval),
        )
        .route(
            openapi_paths::API_REPO_PROJECTED_PAGE_INDEX_TREES_AXUM_PATH,
            get(repo_projected_page_index_trees),
        )
}

/// Create the Studio API router with state already attached.
pub fn studio_router(state: Arc<GatewayState>) -> Router {
    studio_routes().with_state(state)
}

async fn vfs_root_entries(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<Vec<VfsEntry>>, StudioApiError> {
    let entries = vfs::list_root_entries(state.studio.as_ref());
    Ok(Json(entries))
}

async fn vfs_scan(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<VfsScanResult>, StudioApiError> {
    let result = vfs::scan_roots(state.studio.as_ref());
    Ok(Json(result))
}

async fn vfs_entry(
    AxumPath(path): AxumPath<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<VfsEntry>, StudioApiError> {
    let entry = vfs::get_entry(state.studio.as_ref(), path.as_str())?;
    Ok(Json(entry))
}

async fn vfs_cat(
    Query(query): Query<VfsCatQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<VfsContentResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;
    let payload = vfs::read_content(state.studio.as_ref(), path).await?;
    Ok(Json(payload))
}

async fn vfs_resolve(
    Query(query): Query<VfsCatQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<super::types::StudioNavigationTarget>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;
    let payload = vfs::resolve_navigation_target(state.studio.as_ref(), path);
    Ok(Json(payload))
}

async fn node_neighbors(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<NodeNeighbors>, StudioApiError> {
    let payload = graph::node_neighbors(state.as_ref(), id.as_str()).await?;
    Ok(Json(payload))
}

async fn graph_neighbors(
    AxumPath(id): AxumPath<String>,
    Query(query): Query<GraphNeighborsQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<GraphNeighborsResponse>, StudioApiError> {
    let direction = query.direction.unwrap_or_else(|| "both".to_string());
    let hops = query.hops.unwrap_or(2).clamp(1, 5);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let payload =
        graph::graph_neighbors(state.as_ref(), id.as_str(), direction.as_str(), hops, limit)
            .await?;
    Ok(Json(payload))
}

async fn topology_3d(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<super::types::Topology3D>, StudioApiError> {
    let payload = graph::topology_3d(state.as_ref()).await?;
    Ok(Json(payload))
}

async fn analysis_markdown(
    Query(query): Query<MarkdownAnalysisQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<MarkdownAnalysisResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;

    let payload = analysis::analyze_markdown(state.studio.as_ref(), path)
        .await
        .map_err(|error| match error {
            analysis::AnalysisError::UnsupportedContentType(content_type) => {
                StudioApiError::bad_request(
                    "UNSUPPORTED_CONTENT_TYPE",
                    format!("Expected markdown file, received {content_type}"),
                )
            }
            analysis::AnalysisError::Vfs(vfs_error) => StudioApiError::from(vfs_error),
        })?;
    Ok(Json(payload))
}

async fn get_ui_config(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<UiConfig>, StudioApiError> {
    Ok(Json(state.studio.ui_config()))
}

async fn set_ui_config(
    State(state): State<Arc<GatewayState>>,
    Json(config): Json<UiConfig>,
) -> Result<Json<UiConfig>, StudioApiError> {
    state.studio.set_ui_config(config);
    Ok(Json(state.studio.ui_config()))
}

fn required_repo_id(repo: Option<&str>) -> Result<String, StudioApiError> {
    repo.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_REPO", "`repo` is required"))
}

fn required_search_query(query: Option<&str>) -> Result<String, StudioApiError> {
    query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`query` is required"))
}

fn required_page_id(page_id: Option<&str>) -> Result<String, StudioApiError> {
    page_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PAGE_ID", "`page_id` is required"))
}

fn required_node_id(node_id: Option<&str>) -> Result<String, StudioApiError> {
    node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_NODE_ID", "`node_id` is required"))
}

fn parse_repo_sync_mode(mode: Option<&str>) -> Result<RepoSyncMode, StudioApiError> {
    match mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("ensure")
    {
        "ensure" => Ok(RepoSyncMode::Ensure),
        "refresh" => Ok(RepoSyncMode::Refresh),
        "status" => Ok(RepoSyncMode::Status),
        other => Err(StudioApiError::bad_request(
            "INVALID_MODE",
            format!("unsupported repo sync mode `{other}`"),
        )),
    }
}

fn parse_projection_page_kind(
    kind: Option<&str>,
) -> Result<Option<ProjectionPageKind>, StudioApiError> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some("reference") => Ok(Some(ProjectionPageKind::Reference)),
        Some("how_to") => Ok(Some(ProjectionPageKind::HowTo)),
        Some("tutorial") => Ok(Some(ProjectionPageKind::Tutorial)),
        Some("explanation") => Ok(Some(ProjectionPageKind::Explanation)),
        Some(other) => Err(StudioApiError::bad_request(
            "INVALID_KIND",
            format!("unsupported projected page kind `{other}`"),
        )),
    }
}

fn required_projection_page_kind(kind: Option<&str>) -> Result<ProjectionPageKind, StudioApiError> {
    parse_projection_page_kind(kind)?
        .ok_or_else(|| StudioApiError::bad_request("MISSING_KIND", "`kind` is required"))
}

async fn repo_overview(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoOverviewResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_repo_overview(
            &RepoOverviewQuery { repo_id },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_OVERVIEW_PANIC",
            "Repo overview task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_module_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::ModuleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_module_search(
            &ModuleSearchQuery {
                repo_id,
                query: search_query,
                limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_MODULE_SEARCH_PANIC",
            "Repo module search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_symbol_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::SymbolSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_symbol_search(
            &SymbolSearchQuery {
                repo_id,
                query: search_query,
                limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_SYMBOL_SEARCH_PANIC",
            "Repo symbol search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_example_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::ExampleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_example_search(
            &ExampleSearchQuery {
                repo_id,
                query: search_query,
                limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_EXAMPLE_SEARCH_PANIC",
            "Repo example search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_doc_coverage(
    Query(query): Query<RepoDocCoverageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::DocCoverageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_doc_coverage(
            &DocCoverageQuery {
                repo_id,
                module_id: query.module_id,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_DOC_COVERAGE_PANIC",
            "Repo doc coverage task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_sync(
    Query(query): Query<RepoSyncApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoSyncResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let mode = parse_repo_sync_mode(query.mode.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        repo_sync_for_registered_repository(&RepoSyncQuery { repo_id, mode }, &repository, cwd.as_path())
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_SYNC_PANIC",
            "Repo sync task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_pages(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPagesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_repo_projected_pages(
            &RepoProjectedPagesQuery { repo_id },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGES_PANIC",
            "Repo projected pages task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page(&RepoProjectedPageQuery { repo_id, page_id }, &analysis)
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_PANIC",
            "Repo projected page task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_index_tree(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageIndexTreeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_index_tree(
            &RepoProjectedPageIndexTreeQuery { repo_id, page_id },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_INDEX_TREE_PANIC",
            "Repo projected page-index tree task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_index_node(
    Query(query): Query<RepoProjectedPageIndexNodeApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageIndexNodeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = required_node_id(query.node_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_index_node(
            &RepoProjectedPageIndexNodeQuery {
                repo_id,
                page_id,
                node_id,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_INDEX_NODE_PANIC",
            "Repo projected page-index node task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_retrieval_hit(
    Query(query): Query<RepoProjectedRetrievalHitApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedRetrievalHitResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_retrieval_hit(
            &RepoProjectedRetrievalHitQuery {
                repo_id,
                page_id,
                node_id: query.node_id,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_RETRIEVAL_HIT_PANIC",
            "Repo projected retrieval hit task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_retrieval_context(
    Query(query): Query<RepoProjectedRetrievalContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedRetrievalContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_retrieval_context(
            &RepoProjectedRetrievalContextQuery {
                repo_id,
                page_id,
                node_id: query.node_id,
                related_limit,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_RETRIEVAL_CONTEXT_PANIC",
            "Repo projected retrieval context task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_family_context(
    Query(query): Query<RepoProjectedPageFamilyContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageFamilyContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_family_context(
            &RepoProjectedPageFamilyContextQuery {
                repo_id,
                page_id,
                per_kind_limit,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_FAMILY_CONTEXT_PANIC",
            "Repo projected page-family context task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_family_search(
    Query(query): Query<RepoProjectedPageFamilySearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageFamilySearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_repo_projected_page_family_search(
            &RepoProjectedPageFamilySearchQuery {
                repo_id,
                query: search_query,
                kind,
                limit,
                per_kind_limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_FAMILY_SEARCH_PANIC",
            "Repo projected page-family search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_family_cluster(
    Query(query): Query<RepoProjectedPageFamilyClusterApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageFamilyClusterResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let kind = required_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(3).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_family_cluster(
            &RepoProjectedPageFamilyClusterQuery {
                repo_id,
                page_id,
                kind,
                limit,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_FAMILY_CLUSTER_PANIC",
            "Repo projected page-family cluster task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_navigation(
    Query(query): Query<RepoProjectedPageNavigationApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageNavigationResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_navigation(
            &RepoProjectedPageNavigationQuery {
                repo_id,
                page_id,
                node_id: query.node_id,
                family_kind,
                related_limit,
                family_limit,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_NAVIGATION_PANIC",
            "Repo projected page navigation task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_navigation_search(
    Query(query): Query<RepoProjectedPageNavigationSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageNavigationSearchResult>, StudioApiError>
{
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_navigation_search(
            &RepoProjectedPageNavigationSearchQuery {
                repo_id,
                query: search_query,
                kind,
                family_kind,
                limit,
                related_limit,
                family_limit,
            },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_PANIC",
            "Repo projected page navigation search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_index_tree_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageIndexTreeSearchResult>, StudioApiError>
{
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_repo_projected_page_index_tree_search(
            &RepoProjectedPageIndexTreeSearchQuery {
                repo_id,
                query: search_query,
                kind,
                limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_PANIC",
            "Repo projected page-index tree search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_repo_projected_page_search(
            &RepoProjectedPageSearchQuery {
                repo_id,
                query: search_query,
                kind,
                limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_SEARCH_PANIC",
            "Repo projected page search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_retrieval(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedRetrievalResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        Ok::<_, RepoIntelligenceError>(build_repo_projected_retrieval(
            &RepoProjectedRetrievalQuery {
                repo_id,
                query: search_query,
                kind,
                limit,
            },
            &analysis,
        ))
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_RETRIEVAL_PANIC",
            "Repo projected retrieval task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

async fn repo_projected_page_index_trees(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::repo_intelligence::RepoProjectedPageIndexTreesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(state.studio.as_ref(), repo_id.as_str()).map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository(&repository, cwd.as_path())?;
        build_repo_projected_page_index_trees(
            &RepoProjectedPageIndexTreesQuery { repo_id },
            &analysis,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_PROJECTED_PAGE_INDEX_TREES_PANIC",
            "Repo projected page-index trees task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;
    Ok(Json(result))
}

fn sanitize_projects(raw: Vec<UiProjectConfig>) -> Vec<UiProjectConfig> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for project in raw {
        let name = project.name.trim();
        if name.is_empty() {
            continue;
        }
        if !seen.insert(name.to_string()) {
            continue;
        }

        let Some(root) = sanitize_path_like(project.root.as_str()) else {
            continue;
        };

        out.push(UiProjectConfig {
            name: name.to_string(),
            root,
            dirs: sanitize_path_list(project.dirs),
        });
    }
    out
}

fn sanitize_path_list(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in raw {
        let Some(normalized) = pathing::normalize_project_dir_entry(path.as_str()) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

fn sanitize_repo_projects(raw: Vec<UiRepoProjectConfig>) -> Vec<UiRepoProjectConfig> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for project in raw {
        let id = project.id.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let root = project
            .root
            .as_deref()
            .and_then(sanitize_path_like)
            .filter(|value| !value.is_empty());
        let url = project
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let git_ref = project
            .git_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let refresh = project
            .refresh
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut plugin_seen = HashSet::new();
        let plugins = project
            .plugins
            .into_iter()
            .map(|plugin| plugin.trim().to_string())
            .filter(|plugin| !plugin.is_empty())
            .filter(|plugin| plugin_seen.insert(plugin.clone()))
            .collect::<Vec<_>>();
        out.push(UiRepoProjectConfig {
            id: id.to_string(),
            root,
            url,
            git_ref,
            refresh,
            plugins,
        });
    }
    out
}

pub(crate) fn configured_repository(
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

pub(crate) fn configured_repositories(studio: &StudioState) -> Vec<RegisteredRepository> {
    studio
        .configured_repo_projects()
        .into_iter()
        .filter_map(|project| {
            if project.plugins.is_empty() {
                return None;
            }
            let path = project.root.as_deref().and_then(|root| {
                pathing::resolve_path_like(studio.config_root.as_path(), root)
            });
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

fn sanitize_path_like(raw: &str) -> Option<String> {
    pathing::normalize_path_like(raw)
}

fn resolve_studio_config_root(project_root: &Path) -> PathBuf {
    let candidate = PrjDirs::data_home().join("wendao-frontend");
    if candidate.exists() {
        candidate
    } else {
        project_root.to_path_buf()
    }
}

fn graph_include_dirs(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<String> {
    let mut seen = HashSet::new();
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

#[derive(Debug)]
pub(crate) struct StudioApiError {
    status: StatusCode,
    error: ApiError,
}

impl StudioApiError {
    #[cfg(test)]
    pub(crate) fn status(&self) -> StatusCode {
        self.status
    }

    #[cfg(test)]
    pub(crate) fn code(&self) -> &str {
        self.error.code.as_str()
    }

    pub(crate) fn bad_request(code: &str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: ApiError {
                code: code.to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: ApiError {
                code: "NOT_FOUND".to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub(crate) fn internal(
        code: &str,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
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
        (self.status, Json(self.error)).into_response()
    }
}

impl From<vfs::VfsError> for StudioApiError {
    fn from(error: vfs::VfsError) -> Self {
        match error {
            vfs::VfsError::NotFound(path) => Self::not_found(format!("Path not found: {path}")),
            vfs::VfsError::UnknownRoot(root) => {
                Self::bad_request("UNKNOWN_ROOT", format!("Unknown VFS root: {root}"))
            }
            vfs::VfsError::Io(e) => {
                Self::internal("IO_ERROR", "IO error occurred", Some(e.to_string()))
            }
        }
    }
}

fn map_repo_intelligence_error(error: RepoIntelligenceError) -> StudioApiError {
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
