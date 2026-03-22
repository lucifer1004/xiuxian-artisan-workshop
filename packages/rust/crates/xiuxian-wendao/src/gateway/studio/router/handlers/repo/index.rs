use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::gateway::studio::repo_index::{RepoIndexRequest, RepoIndexStatusResponse};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::query::RepoIndexStatusApiQuery;
use super::shared::repo_index_repositories;

/// Repo index enqueue endpoint.
///
/// # Errors
///
/// Returns an error when a requested repository cannot be resolved or when no
/// configured repository is available for indexing.
pub async fn repo_index(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<RepoIndexRequest>,
) -> Result<Json<RepoIndexStatusResponse>, StudioApiError> {
    let repositories = repo_index_repositories(&state, payload.repo.as_deref())?;
    if repositories.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for repo indexing",
        ));
    }
    state
        .studio
        .repo_index
        .ensure_repositories_enqueued(repositories, payload.refresh);
    Ok(Json(
        state
            .studio
            .repo_index
            .status_response(payload.repo.as_deref()),
    ))
}

/// Repo index status endpoint.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn repo_index_status(
    Query(query): Query<RepoIndexStatusApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<RepoIndexStatusResponse>, StudioApiError> {
    Ok(Json(
        state
            .studio
            .repo_index
            .status_response(query.repo.as_deref()),
    ))
}
