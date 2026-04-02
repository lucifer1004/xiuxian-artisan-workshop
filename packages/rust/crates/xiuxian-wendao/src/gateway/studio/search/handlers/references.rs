use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
    LanceUInt64Array,
};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{ReferenceSearchHit, ReferenceSearchResponse};
use xiuxian_wendao_runtime::transport::{
    SEARCH_REFERENCES_ROUTE, SearchFlightRouteProvider, SearchFlightRouteResponse,
};

use super::queries::ReferenceSearchQuery;

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

struct ReferenceSearchFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl std::fmt::Debug for ReferenceSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReferenceSearchFlightRouteProvider")
    }
}

#[async_trait]
impl SearchFlightRouteProvider for ReferenceSearchFlightRouteProvider {
    async fn search_batch(
        &self,
        route: &str,
        query_text: &str,
        limit: usize,
        _intent: Option<&str>,
        _repo_hint: Option<&str>,
    ) -> Result<SearchFlightRouteResponse, String> {
        if route != SEARCH_REFERENCES_ROUTE {
            return Err(format!("unsupported reference Flight route: {route}"));
        }
        let response = load_reference_search_response(
            self.state.as_ref(),
            ReferenceSearchQuery {
                q: Some(query_text.to_string()),
                limit: Some(limit),
            },
        )
        .await
        .map_err(|error| {
            error
                .error
                .details
                .clone()
                .unwrap_or_else(|| format!("{}: {}", error.code(), error.error.message))
        })?;
        build_reference_hits_flight_batch(&response.hits).map(SearchFlightRouteResponse::new)
    }
}

pub(crate) async fn load_reference_search_response_flight_batch(
    state: Arc<GatewayState>,
    query: ReferenceSearchQuery,
) -> Result<LanceRecordBatch, StudioApiError> {
    let raw_query = query.q.clone().unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Reference search requires a non-empty query",
        ));
    }

    let provider = ReferenceSearchFlightRouteProvider { state };
    provider
        .search_batch(
            SEARCH_REFERENCES_ROUTE,
            query_text,
            query.limit.unwrap_or(20).max(1),
            None,
            None,
        )
        .await
        .map(|response| response.batch)
        .map_err(|error| {
            StudioApiError::internal(
                "SEARCH_REFERENCE_FLIGHT_BRIDGE_FAILED",
                "Failed to materialize reference hits through the Flight-backed provider",
                Some(error),
            )
        })
}

fn build_reference_hits_flight_batch(
    hits: &[ReferenceSearchHit],
) -> Result<LanceRecordBatch, String> {
    let names = hits.iter().map(|hit| hit.name.clone()).collect::<Vec<_>>();
    let paths = hits.iter().map(|hit| hit.path.clone()).collect::<Vec<_>>();
    let languages = hits
        .iter()
        .map(|hit| hit.language.clone())
        .collect::<Vec<_>>();
    let crate_names = hits
        .iter()
        .map(|hit| hit.crate_name.clone())
        .collect::<Vec<_>>();
    let project_names = hits
        .iter()
        .map(|hit| hit.project_name.clone().unwrap_or_default())
        .collect::<Vec<_>>();
    let root_labels = hits
        .iter()
        .map(|hit| hit.root_label.clone().unwrap_or_default())
        .collect::<Vec<_>>();
    let navigation_targets_json = hits
        .iter()
        .map(|hit| serde_json::to_string(&hit.navigation_target).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let lines = hits.iter().map(|hit| hit.line as u64).collect::<Vec<_>>();
    let columns = hits.iter().map(|hit| hit.column as u64).collect::<Vec<_>>();
    let line_texts = hits
        .iter()
        .map(|hit| hit.line_text.clone())
        .collect::<Vec<_>>();
    let scores = hits.iter().map(|hit| hit.score).collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("name", LanceDataType::Utf8, false),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("language", LanceDataType::Utf8, false),
            LanceField::new("crateName", LanceDataType::Utf8, false),
            LanceField::new("projectName", LanceDataType::Utf8, false),
            LanceField::new("rootLabel", LanceDataType::Utf8, false),
            LanceField::new("navigationTargetJson", LanceDataType::Utf8, false),
            LanceField::new("line", LanceDataType::UInt64, false),
            LanceField::new("column", LanceDataType::UInt64, false),
            LanceField::new("lineText", LanceDataType::Utf8, false),
            LanceField::new("score", LanceDataType::Float64, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(languages)),
            Arc::new(LanceStringArray::from(crate_names)),
            Arc::new(LanceStringArray::from(project_names)),
            Arc::new(LanceStringArray::from(root_labels)),
            Arc::new(LanceStringArray::from(navigation_targets_json)),
            Arc::new(LanceUInt64Array::from(lines)),
            Arc::new(LanceUInt64Array::from(columns)),
            Arc::new(LanceStringArray::from(line_texts)),
            Arc::new(LanceFloat64Array::from(scores)),
        ],
    )
    .map_err(|error| error.to_string())
}
