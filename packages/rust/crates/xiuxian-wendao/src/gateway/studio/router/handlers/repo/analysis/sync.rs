use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::gateway::studio::router::handlers::repo::command_service::run_repo_sync;
use crate::gateway::studio::router::handlers::repo::parse::parse_repo_sync_mode;
use crate::gateway::studio::router::handlers::repo::required_repo_id;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Repo sync endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, the sync mode is invalid,
/// repository lookup fails, syncing fails, or the background task panics.
pub async fn sync(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoSyncApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoSyncResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let mode = parse_repo_sync_mode(query.mode.as_deref())?;
    let result = run_repo_sync(Arc::clone(&state), repo_id, mode).await?;
    Ok(Json(result))
}
