use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{DocsPageQuery, build_docs_page};
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageApiQuery, required_page_id, required_repo_id,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

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
