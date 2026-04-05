use std::sync::Arc;

use xiuxian_wendao_runtime::transport::SearchFlightRouteResponse;

use super::batch::build_reference_hits_flight_batch;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::queries::ReferenceSearchQuery;
use crate::gateway::studio::types::ReferenceSearchResponse;

pub(crate) async fn load_reference_search_response(
    state: &GatewayState,
    query: ReferenceSearchQuery,
) -> Result<ReferenceSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Reference search requires a non-empty query",
        ));
    }
    state
        .studio
        .ensure_reference_occurrence_index_ready()
        .await?;
    let hits = state
        .studio
        .search_reference_occurrences(query_text, query.limit.unwrap_or(20).max(1))
        .await?;

    Ok(ReferenceSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "references".to_string(),
    })
}

pub(crate) async fn load_reference_search_flight_response(
    state: Arc<GatewayState>,
    query: ReferenceSearchQuery,
) -> Result<SearchFlightRouteResponse, StudioApiError> {
    let response = load_reference_search_response(state.as_ref(), query).await?;
    let app_metadata = serde_json::to_vec(&response).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_REFERENCE_FLIGHT_METADATA_ENCODE_FAILED",
            "Failed to encode reference-search Flight metadata",
            Some(error.to_string()),
        )
    })?;
    build_reference_hits_flight_batch(&response.hits)
        .map(|batch| SearchFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
        .map_err(|error| {
            StudioApiError::internal(
                "SEARCH_REFERENCE_FLIGHT_BATCH_BUILD_FAILED",
                "Failed to build reference-search Flight batch",
                Some(error),
            )
        })
}
