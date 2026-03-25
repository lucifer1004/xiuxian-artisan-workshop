use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    DocsFamilyClusterQuery, DocsFamilyContextQuery, DocsFamilySearchQuery,
    build_docs_family_cluster, build_docs_family_context, build_docs_family_search,
};
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageFamilyClusterApiQuery, RepoProjectedPageFamilyContextApiQuery,
    RepoProjectedPageFamilySearchApiQuery, parse_projection_page_kind, required_page_id,
    required_projection_page_kind, required_repo_id, required_search_query,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

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
