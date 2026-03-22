use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyContextQuery,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageNavigationQuery,
    RepoProjectedPageNavigationSearchQuery, build_repo_projected_page_family_cluster,
    build_repo_projected_page_family_context, build_repo_projected_page_family_search,
    build_repo_projected_page_navigation, build_repo_projected_page_navigation_search,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::parse::{
    parse_projection_page_kind, required_page_id, required_projection_page_kind, required_repo_id,
    required_search_query,
};
use super::query::{
    RepoProjectedPageFamilyClusterApiQuery, RepoProjectedPageFamilyContextApiQuery,
    RepoProjectedPageFamilySearchApiQuery, RepoProjectedPageNavigationApiQuery,
    RepoProjectedPageNavigationSearchApiQuery,
};
use super::shared::with_repo_analysis;

/// Projected page family context endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, family context lookup fails, or the background task panics.
pub async fn projected_page_family_context(
    Query(query): Query<RepoProjectedPageFamilyContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageFamilyContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_FAMILY_CONTEXT_PANIC",
        "Repo projected page-family context task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page_family_context(
                &RepoProjectedPageFamilyContextQuery {
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

/// Projected page family search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_page_family_search(
    Query(query): Query<RepoProjectedPageFamilySearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageFamilySearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let per_kind_limit = query.per_kind_limit.unwrap_or(3);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_FAMILY_SEARCH_PANIC",
        "Repo projected page-family search task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(
                build_repo_projected_page_family_search(
                    &RepoProjectedPageFamilySearchQuery {
                        repo_id,
                        query: search_query,
                        kind,
                        limit,
                        per_kind_limit,
                    },
                    &analysis,
                ),
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page family cluster endpoint.
///
/// # Errors
///
/// Returns an error when `repo`, `page_id`, or `kind` is missing or invalid,
/// repository lookup or analysis fails, family cluster lookup fails, or the
/// background task panics.
pub async fn projected_page_family_cluster(
    Query(query): Query<RepoProjectedPageFamilyClusterApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageFamilyClusterResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let kind = required_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_FAMILY_CLUSTER_PANIC",
        "Repo projected page-family cluster task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page_family_cluster(
                &RepoProjectedPageFamilyClusterQuery {
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

/// Projected page navigation endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, the family kind is
/// invalid, repository lookup or analysis fails, navigation bundle lookup
/// fails, or the background task panics.
pub async fn projected_page_navigation(
    Query(query): Query<RepoProjectedPageNavigationApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageNavigationResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_NAVIGATION_PANIC",
        "Repo projected page navigation task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page_navigation(
                &RepoProjectedPageNavigationQuery {
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

/// Projected page navigation search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, a page-kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
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
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_PANIC",
        "Repo projected page navigation search task failed unexpectedly",
        move |analysis| {
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
        },
    )
    .await?;
    Ok(Json(result))
}
