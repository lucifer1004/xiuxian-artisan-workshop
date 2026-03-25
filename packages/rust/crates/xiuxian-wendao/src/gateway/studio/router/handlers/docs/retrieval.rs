use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    DocsRetrievalContextQuery, DocsRetrievalHitQuery, DocsRetrievalQuery, build_docs_retrieval,
    build_docs_retrieval_context, build_docs_retrieval_hit,
};
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageSearchApiQuery, RepoProjectedRetrievalContextApiQuery,
    RepoProjectedRetrievalHitApiQuery, parse_projection_page_kind, required_page_id,
    required_repo_id, required_search_query,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError, map_repo_intelligence_error};

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
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_retrieval_hit(
                &DocsRetrievalHitQuery {
                    repo: repo_id,
                    page: page_id,
                    node: node_id,
                },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result.map_err(map_repo_intelligence_error)?))
}
