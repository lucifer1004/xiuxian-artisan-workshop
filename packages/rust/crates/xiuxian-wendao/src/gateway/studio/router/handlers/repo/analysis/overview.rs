use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{RepoIntelligenceError, RepoOverviewQuery, build_repo_overview};
use crate::gateway::studio::router::handlers::repo::required_repo_id;
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Repository overview endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup fails,
/// repository analysis fails, or the background task panics.
pub async fn overview(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoOverviewResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_OVERVIEW_PANIC",
        "Repo overview task failed unexpectedly",
        move |analysis| {
            Ok::<_, RepoIntelligenceError>(build_repo_overview(
                &RepoOverviewQuery { repo_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}
