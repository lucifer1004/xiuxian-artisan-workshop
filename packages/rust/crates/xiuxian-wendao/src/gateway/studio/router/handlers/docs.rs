use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::analyzers::{
    DocsFamilyClusterQuery, DocsFamilyContextQuery, DocsFamilySearchQuery, DocsNavigationQuery,
    DocsPageQuery, DocsPlannerItemQuery, DocsPlannerQueueQuery, DocsPlannerRankQuery,
    DocsPlannerSearchQuery, DocsPlannerWorksetQuery, DocsProjectedGapReportQuery,
    DocsRetrievalContextQuery, DocsRetrievalQuery, DocsSearchQuery, build_docs_family_cluster,
    build_docs_family_context, build_docs_family_search, build_docs_navigation,
    build_docs_navigation_search, build_docs_page, build_docs_planner_item,
    build_docs_planner_queue, build_docs_planner_rank, build_docs_planner_search,
    build_docs_planner_workset, build_docs_projected_gap_report, build_docs_retrieval,
    build_docs_retrieval_context, build_docs_search,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::repo::{
    RepoProjectedPageApiQuery, RepoProjectedPageFamilyClusterApiQuery,
    RepoProjectedPageFamilyContextApiQuery, RepoProjectedPageFamilySearchApiQuery,
    RepoProjectedPageNavigationApiQuery, RepoProjectedPageNavigationSearchApiQuery,
    RepoProjectedPageSearchApiQuery, RepoProjectedRetrievalContextApiQuery,
    RepoProjectedRetrievalHitApiQuery, parse_projected_gap_kind, parse_projection_page_kind,
    required_gap_id, required_page_id, required_projection_page_kind, required_repo_id,
    required_search_query, shared::with_repo_analysis,
};

/// Query parameters for docs-facing projected gap lookup.
#[derive(Debug, Deserialize)]
pub struct DocsProjectedGapReportApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
}

/// Query parameters for one docs-facing deterministic planner item.
#[derive(Debug, Deserialize)]
pub struct DocsPlannerItemApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Stable projected gap identifier.
    pub gap_id: Option<String>,
    /// Optional projected-page family to include as a deterministic cluster.
    pub family_kind: Option<String>,
    /// Maximum number of related projected pages to return.
    pub related_limit: Option<usize>,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub family_limit: Option<usize>,
}

/// Query parameters for docs-facing deterministic planner discovery.
#[derive(Debug, Deserialize)]
pub struct DocsPlannerSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Planner search string.
    pub query: Option<String>,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<String>,
    /// Optional projected-page family filter.
    pub page_kind: Option<String>,
    /// Maximum number of planner hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for docs-facing deterministic planner queue shaping.
#[derive(Debug, Deserialize)]
pub struct DocsPlannerQueueApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<String>,
    /// Optional projected-page family filter.
    pub page_kind: Option<String>,
    /// Maximum number of preview gaps to return for each gap kind.
    pub per_kind_limit: Option<usize>,
}

/// Query parameters for docs-facing deterministic planner ranking.
#[derive(Debug, Deserialize)]
pub struct DocsPlannerRankApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<String>,
    /// Optional projected-page family filter.
    pub page_kind: Option<String>,
    /// Maximum number of ranked planner gaps to return.
    pub limit: Option<usize>,
}

/// Query parameters for docs-facing deterministic planner workset opening.
#[derive(Debug, Deserialize)]
pub struct DocsPlannerWorksetApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<String>,
    /// Optional projected-page family filter.
    pub page_kind: Option<String>,
    /// Maximum number of preview gaps to keep for each gap kind.
    pub per_kind_limit: Option<usize>,
    /// Maximum number of planner items to open across the queue preview.
    pub limit: Option<usize>,
    /// Optional projected-page family to include as a deterministic cluster.
    pub family_kind: Option<String>,
    /// Maximum number of related projected pages to return.
    pub related_limit: Option<usize>,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub family_limit: Option<usize>,
}

/// Docs projected gap report endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn projected_gap_report(
    Query(query): Query<DocsProjectedGapReportApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsProjectedGapReportResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PROJECTED_GAP_REPORT_PANIC",
        "Docs projected gap report task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_projected_gap_report(
                &DocsProjectedGapReportQuery { repo_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-item endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `gap_id` is missing, the family filter is invalid,
/// repository lookup or analysis fails, planner-item lookup fails, or the background task panics.
pub async fn planner_item(
    Query(query): Query<DocsPlannerItemApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerItemResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_id = required_gap_id(query.gap_id.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PLANNER_ITEM_PANIC",
        "Docs planner item task failed unexpectedly",
        move |analysis| {
            build_docs_planner_item(
                &DocsPlannerItemQuery {
                    repo_id,
                    gap_id,
                    family_kind,
                    related_limit,
                    family_limit,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, a filter is invalid, repository lookup or
/// analysis fails, or the background task panics.
pub async fn planner_search(
    Query(query): Query<DocsPlannerSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PLANNER_SEARCH_PANIC",
        "Docs planner search task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_planner_search(
                &DocsPlannerSearchQuery {
                    repo_id,
                    query: search_query,
                    gap_kind,
                    page_kind,
                    limit,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-queue endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, a filter is invalid, repository lookup or analysis
/// fails, or the background task panics.
pub async fn planner_queue(
    Query(query): Query<DocsPlannerQueueApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerQueueResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PLANNER_QUEUE_PANIC",
        "Docs planner queue task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_planner_queue(
                &DocsPlannerQueueQuery {
                    repo_id,
                    gap_kind,
                    page_kind,
                    per_kind_limit,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-rank endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, a filter is invalid, repository lookup or analysis
/// fails, or the background task panics.
pub async fn planner_rank(
    Query(query): Query<DocsPlannerRankApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerRankResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PLANNER_RANK_PANIC",
        "Docs planner rank task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_planner_rank(
                &DocsPlannerRankQuery {
                    repo_id,
                    gap_kind,
                    page_kind,
                    limit,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-workset endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, a filter is invalid, repository lookup or analysis
/// fails, one selected planner item cannot be reopened, or the background task panics.
pub async fn planner_workset(
    Query(query): Query<DocsPlannerWorksetApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerWorksetResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3).max(1);
    let limit = query.limit.unwrap_or(3).max(1);
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PLANNER_WORKSET_PANIC",
        "Docs planner workset task failed unexpectedly",
        move |analysis| {
            build_docs_planner_workset(
                &DocsPlannerWorksetQuery {
                    repo_id,
                    gap_kind,
                    page_kind,
                    per_kind_limit,
                    limit,
                    family_kind,
                    related_limit,
                    family_limit,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_SEARCH_PANIC",
        "Docs search task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_search(
                &DocsSearchQuery {
                    repo_id,
                    query: search_query,
                    kind,
                    limit,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs retrieval endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn retrieval(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsRetrievalResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_RETRIEVAL_PANIC",
        "Docs retrieval task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_retrieval(
                &DocsRetrievalQuery {
                    repo_id,
                    query: search_query,
                    kind,
                    limit,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs retrieval context endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, retrieval context lookup fails, or the background task
/// panics.
pub async fn retrieval_context(
    Query(query): Query<RepoProjectedRetrievalContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsRetrievalContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let related_limit = query.related_limit.unwrap_or(5);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_RETRIEVAL_CONTEXT_PANIC",
        "Docs retrieval context task failed unexpectedly",
        move |analysis| {
            build_docs_retrieval_context(
                &DocsRetrievalContextQuery {
                    repo_id,
                    page_id,
                    node_id,
                    related_limit,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs retrieval hit endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, retrieval-hit lookup fails, or the background task panics.
pub async fn retrieval_hit(
    Query(query): Query<RepoProjectedRetrievalHitApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsRetrievalHitResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_RETRIEVAL_HIT_PANIC",
        "Docs retrieval hit task failed unexpectedly",
        move |analysis| {
            crate::analyzers::build_docs_retrieval_hit(
                &crate::analyzers::DocsRetrievalHitQuery {
                    repo: repo_id,
                    page: page_id,
                    node: node_id,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs page endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, projected page lookup fails, or the background task panics.
pub async fn page(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PAGE_PANIC",
        "Docs page task failed unexpectedly",
        move |analysis| build_docs_page(&DocsPageQuery { repo_id, page_id }, &analysis),
    )
    .await?;
    Ok(Json(result))
}

/// Docs family context endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, family-context lookup fails, or the background task panics.
pub async fn family_context(
    Query(query): Query<RepoProjectedPageFamilyContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsFamilyContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_FAMILY_CONTEXT_PANIC",
        "Docs family context task failed unexpectedly",
        move |analysis| {
            build_docs_family_context(
                &DocsFamilyContextQuery {
                    repo_id,
                    page_id,
                    per_kind_limit,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs family search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task panics.
pub async fn family_search(
    Query(query): Query<RepoProjectedPageFamilySearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsFamilySearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_FAMILY_SEARCH_PANIC",
        "Docs family search task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_family_search(
                &DocsFamilySearchQuery {
                    repo_id,
                    query: search_query,
                    kind,
                    limit,
                    per_kind_limit,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs family cluster endpoint.
///
/// # Errors
///
/// Returns an error when `repo`, `page_id`, or `kind` is missing or invalid,
/// repository lookup or analysis fails, family-cluster lookup fails, or the
/// background task panics.
pub async fn family_cluster(
    Query(query): Query<RepoProjectedPageFamilyClusterApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsFamilyClusterResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let kind = required_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_FAMILY_CLUSTER_PANIC",
        "Docs family cluster task failed unexpectedly",
        move |analysis| {
            build_docs_family_cluster(
                &DocsFamilyClusterQuery {
                    repo_id,
                    page_id,
                    kind,
                    limit,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs navigation endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, the family kind is
/// invalid, repository lookup or analysis fails, navigation bundle lookup
/// fails, or the background task panics.
pub async fn navigation(
    Query(query): Query<RepoProjectedPageNavigationApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsNavigationResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_NAVIGATION_PANIC",
        "Docs navigation task failed unexpectedly",
        move |analysis| {
            build_docs_navigation(
                &DocsNavigationQuery {
                    repo_id,
                    page_id,
                    node_id,
                    family_kind,
                    related_limit,
                    family_limit,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs navigation search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, a page-kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn navigation_search(
    Query(query): Query<RepoProjectedPageNavigationSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsNavigationSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_NAVIGATION_SEARCH_PANIC",
        "Docs navigation search task failed unexpectedly",
        move |analysis| {
            build_docs_navigation_search(
                &crate::analyzers::DocsNavigationSearchQuery {
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
        },
    )
    .await?;
    Ok(Json(result))
}
