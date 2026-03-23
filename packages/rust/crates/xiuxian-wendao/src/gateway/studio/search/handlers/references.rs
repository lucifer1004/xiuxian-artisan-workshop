use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::ReferenceSearchResponse;

use super::queries::ReferenceSearchQuery;

pub async fn search_references(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<ReferenceSearchQuery>,
) -> Result<Json<ReferenceSearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Reference search requires a non-empty query",
        ));
    }
    state.studio.ensure_reference_occurrence_index_started()?;
    let hits = state
        .studio
        .search_reference_occurrences(query_text, query.limit.unwrap_or(20).max(1))
        .await?;

    Ok(Json(ReferenceSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "references".to_string(),
    }))
}
