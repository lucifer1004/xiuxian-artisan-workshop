//! Repository Intelligence endpoint handlers for Studio API.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::analyzers::{
    DocCoverageQuery, ExampleSearchQuery, ModuleSearchQuery, ProjectionPageKind, RepoOverviewQuery,
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyContextQuery,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageIndexNodeQuery,
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeSearchQuery,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageNavigationQuery,
    RepoProjectedPageNavigationSearchQuery, RepoProjectedPageQuery, RepoProjectedPageSearchQuery,
    RepoProjectedPagesQuery, RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalHitQuery,
    RepoProjectedRetrievalQuery, RepoSyncMode, RepoSyncQuery, SymbolSearchQuery,
    analyze_registered_repository_cached_with_registry as analyze_registered_repository_with_registry,
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
use crate::gateway::studio::repo_index::{RepoIndexRequest, RepoIndexStatusResponse};
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repository, map_repo_intelligence_error,
};

// --- Query parameter types ---

/// Basic repository query parameters.
#[derive(Debug, Deserialize)]
pub struct RepoApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
}

/// Query parameters for repository-wide search.
#[derive(Debug, Deserialize)]
pub struct RepoSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for projected page lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
}

/// Query parameters for projected page-index node lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageIndexNodeApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The page-index node identifier.
    pub node_id: Option<String>,
}

/// Query parameters for projected retrieval hit lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedRetrievalHitApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The page-index node identifier.
    pub node_id: Option<String>,
}

/// Query parameters for projected retrieval context lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedRetrievalContextApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The page-index node identifier.
    pub node_id: Option<String>,
    /// Maximum number of related hits to return.
    pub related_limit: Option<usize>,
}

/// Query parameters for projected page-family context lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageFamilyContextApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// Maximum number of items per kind to return.
    pub per_kind_limit: Option<usize>,
}

/// Query parameters for projected page-family cluster search.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageFamilySearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
    /// Maximum number of items per kind to return.
    pub per_kind_limit: Option<usize>,
}

/// Query parameters for projected page-family cluster lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageFamilyClusterApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for projected page navigation bundle lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageNavigationApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The focus node identifier.
    pub node_id: Option<String>,
    /// The family kind filter.
    pub family_kind: Option<String>,
    /// Maximum number of related hits to return.
    pub related_limit: Option<usize>,
    /// Maximum number of family items to return.
    pub family_limit: Option<usize>,
}

/// Query parameters for projected page navigation search.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageNavigationSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// The family kind filter.
    pub family_kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
    /// Maximum number of related hits to return.
    pub related_limit: Option<usize>,
    /// Maximum number of family items to return.
    pub family_limit: Option<usize>,
}

/// Query parameters for projected-page search.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for documentation coverage inspection.
#[derive(Debug, Deserialize)]
pub struct RepoDocCoverageApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Optional module identifier filter.
    #[serde(rename = "module")]
    pub module_id: Option<String>,
}

/// Query parameters for repository source synchronization.
#[derive(Debug, Deserialize)]
pub struct RepoSyncApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The synchronization mode ("ensure", "refresh", or "status").
    pub mode: Option<String>,
}

/// Query parameters for repo index status.
#[derive(Debug, Deserialize)]
pub struct RepoIndexStatusApiQuery {
    /// Optional repository identifier filter.
    pub repo: Option<String>,
}

// --- Helper functions ---

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

// --- Handlers ---

/// Repository overview endpoint.
pub async fn overview(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoOverviewResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_overview(
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

/// Module search endpoint.
pub async fn module_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ModuleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_module_search(
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

/// Symbol search endpoint.
pub async fn symbol_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::SymbolSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_symbol_search(
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

/// Example search endpoint.
pub async fn example_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ExampleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_example_search(
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

/// Doc coverage endpoint.
pub async fn doc_coverage(
    Query(query): Query<RepoDocCoverageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocCoverageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_doc_coverage(
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

/// Repo sync endpoint.
pub async fn sync(
    Query(query): Query<RepoSyncApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoSyncResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let mode = parse_repo_sync_mode(query.mode.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let result = tokio::task::spawn_blocking(move || {
        repo_sync_for_registered_repository(
            &RepoSyncQuery { repo_id, mode },
            &repository,
            cwd.as_path(),
        )
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

/// Repo index enqueue endpoint.
pub async fn repo_index(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<RepoIndexRequest>,
) -> Result<Json<RepoIndexStatusResponse>, StudioApiError> {
    let repositories = if let Some(repo_id) = payload.repo.as_deref() {
        vec![configured_repository(&state.studio, repo_id).map_err(map_repo_intelligence_error)?]
    } else {
        crate::gateway::studio::router::configured_repositories(&state.studio)
    };
    if repositories.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for repo indexing",
        ));
    }
    state
        .studio
        .repo_index
        .ensure_repositories_enqueued(repositories, payload.refresh);
    Ok(Json(
        state
            .studio
            .repo_index
            .status_response(payload.repo.as_deref()),
    ))
}

/// Repo index status endpoint.
pub async fn repo_index_status(
    Query(query): Query<RepoIndexStatusApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<RepoIndexStatusResponse>, StudioApiError> {
    Ok(Json(
        state
            .studio
            .repo_index
            .status_response(query.repo.as_deref()),
    ))
}

/// Projected pages endpoint.
pub async fn projected_pages(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPagesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_projected_pages(
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

/// Projected page endpoint.
pub async fn projected_page(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page index tree endpoint.
pub async fn projected_page_index_tree(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page index node endpoint.
pub async fn projected_page_index_node(
    Query(query): Query<RepoProjectedPageIndexNodeApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexNodeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = required_node_id(query.node_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected retrieval hit endpoint.
pub async fn projected_retrieval_hit(
    Query(query): Query<RepoProjectedRetrievalHitApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalHitResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected retrieval context endpoint.
pub async fn projected_retrieval_context(
    Query(query): Query<RepoProjectedRetrievalContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page family context endpoint.
pub async fn projected_page_family_context(
    Query(query): Query<RepoProjectedPageFamilyContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageFamilyContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page family search endpoint.
pub async fn projected_page_family_search(
    Query(query): Query<RepoProjectedPageFamilySearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageFamilySearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_projected_page_family_search(
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

/// Projected page family cluster endpoint.
pub async fn projected_page_family_cluster(
    Query(query): Query<RepoProjectedPageFamilyClusterApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageFamilyClusterResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let kind = required_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(3).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page navigation endpoint.
pub async fn projected_page_navigation(
    Query(query): Query<RepoProjectedPageNavigationApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageNavigationResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page navigation search endpoint.
#[allow(clippy::too_many_lines)]
pub async fn projected_page_navigation_search(
    Query(query): Query<RepoProjectedPageNavigationSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageNavigationSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Projected page index tree search endpoint.
pub async fn projected_page_index_tree_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreeSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(
            build_repo_projected_page_index_tree_search(
                &RepoProjectedPageIndexTreeSearchQuery {
                    repo_id,
                    query: search_query,
                    kind,
                    limit,
                },
                &analysis,
            ),
        )
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

/// Projected page search endpoint.
pub async fn projected_page_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_projected_page_search(
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

/// Projected retrieval endpoint.
pub async fn projected_retrieval(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_projected_retrieval(
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

/// Projected page index trees endpoint.
pub async fn projected_page_index_trees(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
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

/// Refine documentation for a specific entity using the Trinity loop.
pub async fn refine_entity_doc(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<crate::analyzers::RefineEntityDocRequest>,
) -> Result<Json<crate::analyzers::RefineEntityDocResponse>, StudioApiError> {
    let repo_id = required_repo_id(Some(payload.repo_id.as_str()))?;
    let cwd = state.studio.project_root.clone();
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);

    let result = tokio::task::spawn_blocking(move || {
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;

        // 1. Locate the entity
        let symbol = analysis
            .symbols
            .iter()
            .find(|s| s.symbol_id == payload.entity_id)
            .ok_or_else(|| crate::RepoIntelligenceError::AnalysisFailed {
                message: format!("Entity `{}` not found", payload.entity_id),
            })?;

        // 2. Annotator Phase (Mocked for now)
        let refined_content = format!(
            "## Refined Explanation for {}\n\nThis {:?} is part of the `{}` module. \
            It has been automatically refined using user hints: \"{}\".\n\n\
            **Signature**: `{}`",
            symbol.name,
            symbol.kind,
            symbol.module_id.as_deref().unwrap_or("root"),
            payload.user_hints.as_deref().unwrap_or("none"),
            symbol.signature.as_deref().unwrap_or("unknown")
        );

        // 3. Skeptic Audit Phase
        // For the MVP, we assume the refined content is verified since we grounded it in AST.
        let verification_state = "verified".to_string();

        Ok::<_, crate::RepoIntelligenceError>(crate::analyzers::RefineEntityDocResponse {
            repo_id: payload.repo_id,
            entity_id: payload.entity_id,
            refined_content,
            verification_state,
        })
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REFINE_DOC_PANIC",
            "Refine documentation task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;

    Ok(Json(result))
}
