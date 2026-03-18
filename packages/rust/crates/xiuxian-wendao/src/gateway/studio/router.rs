use std::collections::HashSet;
use std::path::PathBuf;
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
use tokio::sync::OnceCell;
use xiuxian_io::PrjDirs;
use xiuxian_zhenfa::ZhenfaSignal;

use crate::link_graph::LinkGraphIndex;
use crate::skill_vfs::SkillVfsResolver;
use crate::unified_symbol::UnifiedSymbolIndex;

use super::types::{
    ApiError, AstSearchHit, GraphNeighborsResponse, NodeNeighbors, UiConfig, VfsContentResponse,
    VfsEntry, VfsScanResult,
};
use super::{graph, search, vfs};

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: PathBuf,
    pub(crate) data_root: PathBuf,
    pub(crate) knowledge_root: PathBuf,
    pub(crate) internal_skill_root: PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: OnceCell<Arc<LinkGraphIndex>>,
    pub(crate) symbol_index: OnceCell<Arc<UnifiedSymbolIndex>>,
    pub(crate) ast_index: OnceCell<Arc<Vec<AstSearchHit>>>,
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
        if let Some(index) = &self.index {
            return Ok(Arc::clone(index));
        }
        self.studio.graph_index().await
    }
}

impl StudioState {
    /// Create a new `StudioState` with default configuration.
    #[must_use]
    pub fn new() -> Self {
        let project_root = PrjDirs::project_root();
        let data_root = PrjDirs::data_home();
        let knowledge_root = data_root.join("knowledge");
        let internal_skill_root = SkillVfsResolver::resolve_runtime_internal_root_with(
            project_root.as_path(),
            std::env::var("PRJ_INTERNAL_SKILLS_DIR").ok().as_deref(),
        );
        Self {
            project_root,
            data_root,
            knowledge_root,
            internal_skill_root,
            ui_config: Arc::new(RwLock::new(UiConfig {
                index_paths: Vec::new(),
            })),
            graph_index: OnceCell::new(),
            symbol_index: OnceCell::new(),
            ast_index: OnceCell::new(),
        }
    }

    pub(crate) fn ui_config(&self) -> UiConfig {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn set_ui_config(&self, config: UiConfig) {
        let sanitized = sanitize_index_paths(config.index_paths);
        let mut guard = self
            .ui_config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.index_paths = sanitized;
    }

    pub(crate) fn configured_index_paths(&self) -> Vec<String> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .index_paths
            .clone()
    }

    pub(crate) async fn graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        let knowledge_root = self.knowledge_root.clone();
        let index = self
            .graph_index
            .get_or_try_init(|| async move {
                let build = tokio::task::spawn_blocking(move || {
                    LinkGraphIndex::build(knowledge_root.as_path())
                })
                .await
                .map_err(|error| {
                    StudioApiError::internal(
                        "LINK_GRAPH_BUILD_PANIC",
                        "Failed to build link graph index",
                        Some(error.to_string()),
                    )
                })?;
                let index = build.map_err(|error| {
                    StudioApiError::internal(
                        "LINK_GRAPH_BUILD_FAILED",
                        "Failed to build link graph index",
                        Some(error),
                    )
                })?;
                Ok::<Arc<LinkGraphIndex>, StudioApiError>(Arc::new(index))
            })
            .await?;
        Ok(Arc::clone(index))
    }

    pub(crate) async fn symbol_index(&self) -> Result<Arc<UnifiedSymbolIndex>, StudioApiError> {
        let project_root = self.project_root.clone();
        let index = self
            .symbol_index
            .get_or_try_init(|| async move {
                let build = tokio::task::spawn_blocking(move || {
                    search::build_symbol_index(project_root.as_path())
                })
                .await
                .map_err(|error| {
                    StudioApiError::internal(
                        "SYMBOL_INDEX_BUILD_PANIC",
                        "Failed to build studio symbol index",
                        Some(error.to_string()),
                    )
                })?;
                let index = build.map_err(|error| {
                    StudioApiError::internal(
                        "SYMBOL_INDEX_BUILD_FAILED",
                        "Failed to build studio symbol index",
                        Some(error),
                    )
                })?;
                Ok::<Arc<UnifiedSymbolIndex>, StudioApiError>(Arc::new(index))
            })
            .await?;
        Ok(Arc::clone(index))
    }

    pub(crate) async fn ast_index(&self) -> Result<Arc<Vec<AstSearchHit>>, StudioApiError> {
        let project_root = self.project_root.clone();
        let index = self
            .ast_index
            .get_or_try_init(|| async move {
                let build = tokio::task::spawn_blocking(move || {
                    search::build_ast_index(project_root.as_path())
                })
                .await
                .map_err(|error| {
                    StudioApiError::internal(
                        "AST_INDEX_BUILD_PANIC",
                        "Failed to build studio AST index",
                        Some(error.to_string()),
                    )
                })?;
                let index = build.map_err(|error| {
                    StudioApiError::internal(
                        "AST_INDEX_BUILD_FAILED",
                        "Failed to build studio AST index",
                        Some(error),
                    )
                })?;
                Ok::<Arc<Vec<AstSearchHit>>, StudioApiError>(Arc::new(index))
            })
            .await?;
        Ok(Arc::clone(index))
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

/// Create the Studio API router with all endpoints.
///
/// # Endpoints
///
/// - `GET /api/vfs` - List root entries
/// - `GET /api/vfs/scan` - Scan all VFS roots
/// - `GET /api/vfs/cat?path=` - Read file content
/// - `GET /api/vfs/{*path}` - Get single entry
/// - `GET /api/neighbors/{*id}` - Get node neighbors
/// - `GET /api/graph/neighbors/{*id}` - Get graph neighbors
/// - `GET /api/topology/3d` - Get deterministic graph topology payload
/// - `GET /api/search` - Search knowledge base
/// - `GET /api/search/ast` - Search AST definitions
/// - `GET /api/search/references` - Search symbol references and usages
/// - `GET /api/search/symbols` - Search project symbols
/// - `GET /api/search/autocomplete` - Search autocomplete suggestions
/// - `GET/POST /api/ui/config` - UI configuration
pub fn studio_routes() -> Router<Arc<GatewayState>> {
    Router::new()
        .route("/api/vfs", get(vfs_root_entries))
        .route("/api/vfs/scan", get(vfs_scan))
        .route("/api/vfs/cat", get(vfs_cat))
        .route("/api/vfs/{*path}", get(vfs_entry))
        .route("/api/neighbors/{*id}", get(node_neighbors))
        .route("/api/graph/neighbors/{*id}", get(graph_neighbors))
        .route("/api/topology/3d", get(topology_3d))
        .route("/api/search", get(search::search_knowledge))
        .route("/api/search/ast", get(search::search_ast))
        .route("/api/search/references", get(search::search_references))
        .route("/api/search/symbols", get(search::search_symbols))
        .route("/api/search/autocomplete", get(search::search_autocomplete))
        .route("/api/ui/config", get(get_ui_config).post(set_ui_config))
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

fn sanitize_index_paths(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in raw {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed
            .replace('\\', "/")
            .trim_end_matches('/')
            .trim_start_matches("./")
            .to_string();
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
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
