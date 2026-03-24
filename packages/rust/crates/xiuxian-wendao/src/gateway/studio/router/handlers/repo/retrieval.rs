use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::cache::{
    RepositorySearchQueryCacheKey, load_cached_repository_search_result,
    store_cached_repository_search_result,
};
use crate::analyzers::service::{
    build_repo_projected_page_search_with_artifacts, repository_search_artifacts,
};
use crate::analyzers::{
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageSearchQuery,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalHitQuery,
    RepoProjectedRetrievalQuery, build_repo_projected_page_index_tree_search,
    build_repo_projected_retrieval, build_repo_projected_retrieval_context,
    build_repo_projected_retrieval_hit,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::search::FuzzySearchOptions;

use super::parse::{
    parse_projection_page_kind, required_page_id, required_repo_id, required_search_query,
};
use super::query::{
    RepoProjectedPageSearchApiQuery, RepoProjectedRetrievalContextApiQuery,
    RepoProjectedRetrievalHitApiQuery,
};
use super::shared::{with_repo_analysis, with_repo_cached_analysis_bundle};

/// Projected retrieval hit endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, hit lookup fails, or the background task panics.
pub async fn projected_retrieval_hit(
    Query(query): Query<RepoProjectedRetrievalHitApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalHitResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_RETRIEVAL_HIT_PANIC",
        "Repo projected retrieval hit task failed unexpectedly",
        move |analysis| {
            build_repo_projected_retrieval_hit(
                &RepoProjectedRetrievalHitQuery {
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

/// Projected retrieval context endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, retrieval context lookup fails, or the background task
/// panics.
pub async fn projected_retrieval_context(
    Query(query): Query<RepoProjectedRetrievalContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let related_limit = query.related_limit.unwrap_or(5);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_RETRIEVAL_CONTEXT_PANIC",
        "Repo projected retrieval context task failed unexpectedly",
        move |analysis| {
            build_repo_projected_retrieval_context(
                &RepoProjectedRetrievalContextQuery {
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

/// Projected page index tree search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_page_index_tree_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreeSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_PANIC",
        "Repo projected page-index tree search task failed unexpectedly",
        move |analysis| {
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
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_page_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_cached_analysis_bundle(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_PAGE_SEARCH_PANIC",
        "Repo projected page search task failed unexpectedly",
        move |cached| {
            let query = RepoProjectedPageSearchQuery {
                repo_id,
                query: search_query,
                kind,
                limit,
            };
            let filter = query
                .kind
                .map(|kind| format!("{kind:?}").to_ascii_lowercase());
            let cache_key = RepositorySearchQueryCacheKey::new(
                &cached.cache_key,
                "repo.projected-page-search",
                query.query.as_str(),
                filter,
                FuzzySearchOptions::document_search(),
                query.limit,
            );
            if let Some(result) = load_cached_repository_search_result(&cache_key)? {
                return Ok(result);
            }

            let artifacts = repository_search_artifacts(&cached.cache_key, &cached.analysis)?;
            let result = build_repo_projected_page_search_with_artifacts(
                &query,
                &cached.analysis,
                artifacts.as_ref(),
            );
            store_cached_repository_search_result(cache_key, &result)?;
            Ok::<_, crate::analyzers::RepoIntelligenceError>(result)
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected retrieval endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_retrieval(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_PROJECTED_RETRIEVAL_PANIC",
        "Repo projected retrieval task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_projected_retrieval(
                &RepoProjectedRetrievalQuery {
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
