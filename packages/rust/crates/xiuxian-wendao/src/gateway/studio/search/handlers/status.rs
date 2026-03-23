use std::sync::Arc;

use axum::{Json, extract::State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::SearchIndexStatusResponse;

/// Studio search-plane status endpoint.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn search_index_status(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SearchIndexStatusResponse>, StudioApiError> {
    Ok(Json(state.studio.search_index_status().await))
}
