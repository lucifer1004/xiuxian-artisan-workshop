use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{DocCoverageQuery, RepoIntelligenceError, build_doc_coverage};
use crate::gateway::studio::router::handlers::repo::required_repo_id;
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Doc coverage endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn doc_coverage(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoDocCoverageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocCoverageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let module_id = query.module_id;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_DOC_COVERAGE_PANIC",
        "Repo doc coverage task failed unexpectedly",
        move |analysis| {
            Ok::<_, RepoIntelligenceError>(build_doc_coverage(
                &DocCoverageQuery { repo_id, module_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}
