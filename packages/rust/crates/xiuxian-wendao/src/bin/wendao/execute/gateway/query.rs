//! Gateway shared-query compatibility route.

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use xiuxian_wendao::search::queries::{
    SearchQueryService,
    rest::{RestQueryPayload, RestQueryRequest, query_rest_payload},
};

use crate::execute::gateway::shared::AppState;

/// Compatibility HTTP route used by bounded external query clients.
pub(crate) const GATEWAY_QUERY_AXUM_PATH: &str = "/query";

/// Execute one shared REST query request through the gateway.
pub(crate) async fn query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RestQueryRequest>,
) -> Result<Json<RestQueryPayload>, (StatusCode, Json<serde_json::Value>)> {
    let service = SearchQueryService::new(state.studio.search_plane_service());
    query_rest_payload(&service, &request)
        .await
        .map(Json)
        .map_err(|details| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "failed to execute shared query request",
                    "code": "QUERY_EXECUTION_FAILED",
                    "details": details,
                })),
            )
        })
}
