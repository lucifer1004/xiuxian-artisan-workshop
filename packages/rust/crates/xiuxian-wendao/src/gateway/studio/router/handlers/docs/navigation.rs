use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::DocsNavigationQuery;
use crate::analyzers::{build_docs_navigation, build_docs_navigation_search};
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageNavigationApiQuery, RepoProjectedPageNavigationSearchApiQuery,
    parse_projection_page_kind, required_page_id, required_repo_id, required_search_query,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

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
