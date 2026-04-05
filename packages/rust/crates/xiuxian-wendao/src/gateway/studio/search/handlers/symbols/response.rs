use xiuxian_wendao_runtime::transport::SearchFlightRouteResponse;

use super::batch::build_symbol_hits_flight_batch;
use super::hit::symbol_search_hit;
use super::matcher::build_project_glob_matcher;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::queries::SymbolSearchQuery;
use crate::gateway::studio::symbol_index::SymbolIndexPhase;
use crate::gateway::studio::types::{SymbolSearchHit, SymbolSearchResponse};

pub(crate) fn load_symbol_search_response(
    state: &GatewayState,
    query: SymbolSearchQuery,
) -> Result<SymbolSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Symbol search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let status = state.studio.symbol_index_status()?;
    let Some(index) = state.studio.current_symbol_index() else {
        return Ok(SymbolSearchResponse {
            query: query_text.to_string(),
            hit_count: 0,
            selected_scope: "project".to_string(),
            partial: true,
            indexing_state: Some(status.phase.as_str().to_string()),
            index_error: status.last_error,
            hits: Vec::new(),
        });
    };
    let projects = state.studio.configured_projects();
    let glob_matcher = build_project_glob_matcher(projects.as_slice());
    let mut hits: Vec<SymbolSearchHit> = index
        .search_unified(query_text, limit)
        .into_iter()
        .enumerate()
        .map(|(rank, symbol)| {
            symbol_search_hit(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
                symbol,
                rank,
            )
        })
        .filter(|hit| {
            glob_matcher
                .as_ref()
                .is_none_or(|matcher| matcher.is_match(hit.path.as_str()))
        })
        .collect();
    hits.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });

    Ok(SymbolSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        selected_scope: "project".to_string(),
        partial: false,
        indexing_state: Some(SymbolIndexPhase::Ready.as_str().to_string()),
        index_error: None,
        hits: {
            hits.truncate(limit);
            hits
        },
    })
}

pub(crate) fn load_symbol_search_flight_response(
    state: &GatewayState,
    query: SymbolSearchQuery,
) -> Result<SearchFlightRouteResponse, StudioApiError> {
    let response = load_symbol_search_response(state, query)?;
    let app_metadata = serde_json::to_vec(&response).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_SYMBOL_FLIGHT_METADATA_ENCODE_FAILED",
            "Failed to encode symbol-search Flight metadata",
            Some(error.to_string()),
        )
    })?;
    build_symbol_hits_flight_batch(&response.hits)
        .map(|batch| SearchFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
        .map_err(|error| {
            StudioApiError::internal(
                "SEARCH_SYMBOL_FLIGHT_BATCH_BUILD_FAILED",
                "Failed to build symbol-search Flight batch",
                Some(error),
            )
        })
}
