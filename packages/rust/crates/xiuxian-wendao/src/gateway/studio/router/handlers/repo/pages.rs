use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexTreeQuery,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageQuery, RepoProjectedPagesQuery,
    build_repo_projected_page, build_repo_projected_page_index_node,
    build_repo_projected_page_index_tree, build_repo_projected_page_index_trees,
    build_repo_projected_pages,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::parse::{required_node_id, required_page_id, required_repo_id};
use super::query::{RepoApiQuery, RepoProjectedPageApiQuery, RepoProjectedPageIndexNodeApiQuery};
use super::shared::with_repo_analysis;

/// Projected pages endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn projected_pages(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPagesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGES_PANIC",
        "Repo projected pages task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_projected_pages(
                &RepoProjectedPagesQuery { repo_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, projected page lookup fails, or the background task panics.
pub async fn projected_page(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_PANIC",
        "Repo projected page task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page(&RepoProjectedPageQuery { repo_id, page_id }, &analysis)
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index tree endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, page-index tree lookup fails, or the background task
/// panics.
pub async fn projected_page_index_tree(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_INDEX_TREE_PANIC",
        "Repo projected page-index tree task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page_index_tree(
                &RepoProjectedPageIndexTreeQuery { repo_id, page_id },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index node endpoint.
///
/// # Errors
///
/// Returns an error when `repo`, `page_id`, or `node_id` is missing,
/// repository lookup or analysis fails, page-index node lookup fails, or the
/// background task panics.
pub async fn projected_page_index_node(
    Query(query): Query<RepoProjectedPageIndexNodeApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexNodeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = required_node_id(query.node_id.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_INDEX_NODE_PANIC",
        "Repo projected page-index node task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page_index_node(
                &RepoProjectedPageIndexNodeQuery {
                    repo_id,
                    page_id,
                    node_id,
                },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index trees endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, page-index tree construction fails, or the background task panics.
pub async fn projected_page_index_trees(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_INDEX_TREES_PANIC",
        "Repo projected page-index trees task failed unexpectedly",
        move |analysis| {
            build_repo_projected_page_index_trees(
                &RepoProjectedPageIndexTreesQuery { repo_id },
                &analysis,
            )
        },
    )
    .await?;
    Ok(Json(result))
}
