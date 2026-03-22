use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::ReferenceSearchResponse;

use super::super::source_index;
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

    let ast_index = state.studio.ast_index().await?;
    let projects = state.studio.configured_projects();
    let hits = source_index::build_reference_hits(
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        projects.as_slice(),
        ast_index.as_slice(),
        query_text,
        query.limit.unwrap_or(20).max(1),
    )
    .map_err(|detail| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_BUILD_FAILED",
            "Failed to build Studio reference search results",
            Some(detail),
        )
    })?;

    Ok(Json(ReferenceSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "references".to_string(),
    }))
}
