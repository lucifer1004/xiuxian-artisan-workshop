use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::Response,
};

use crate::gateway::studio::router::retrieval_arrow::retrieval_chunks_arrow_response;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::MarkdownAnalysisResponse;

use crate::gateway::studio::router::handlers::analysis::service::load_markdown_analysis_response;
use crate::gateway::studio::router::handlers::analysis::types::MarkdownAnalysisQuery;

/// Analyzes markdown file structure.
///
/// # Errors
///
/// Returns an error when `path` is missing or when markdown analysis fails.
pub async fn markdown(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<MarkdownAnalysisQuery>,
) -> Result<Json<MarkdownAnalysisResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;

    let response = load_markdown_analysis_response(state.as_ref(), path).await?;

    Ok(Json(response))
}

/// Returns shared retrieval chunks for markdown analysis as Arrow IPC.
///
/// # Errors
///
/// Returns an error when `path` is missing, markdown analysis fails, or Arrow
/// IPC encoding fails.
pub async fn markdown_retrieval_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<MarkdownAnalysisQuery>,
) -> Result<Response, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;
    let response = load_markdown_analysis_response(state.as_ref(), path).await?;
    retrieval_chunks_arrow_response(&response.retrieval_atoms)
        .map_err(|error| StudioApiError::internal("MARKDOWN_RETRIEVAL_ARROW_FAILED", error, None))
}
