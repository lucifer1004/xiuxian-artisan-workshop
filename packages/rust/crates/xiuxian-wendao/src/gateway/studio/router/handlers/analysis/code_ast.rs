use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::Response,
};

use crate::gateway::studio::router::retrieval_arrow::retrieval_chunks_arrow_response;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::CodeAstAnalysisResponse;

use crate::gateway::studio::router::handlers::analysis::service::load_code_ast_analysis_response;
use crate::gateway::studio::router::handlers::analysis::types::CodeAstAnalysisQuery;

/// Analyzes code file AST and projections.
///
/// # Errors
///
/// Returns an error when `repo` or `path` is missing, when repository analysis
/// fails, or when the background analysis task panics.
pub async fn code_ast(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<CodeAstAnalysisQuery>,
) -> Result<Json<CodeAstAnalysisResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;

    let repo_id = query
        .repo
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_REPO", "`repo` is required"))?;

    let payload =
        load_code_ast_analysis_response(state.as_ref(), path, repo_id, query.line).await?;

    Ok(Json(payload))
}

/// Returns shared retrieval chunks for code AST analysis as Arrow IPC.
///
/// # Errors
///
/// Returns an error when `repo` or `path` is missing, repository analysis
/// fails, or Arrow IPC encoding fails.
pub async fn code_ast_retrieval_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<CodeAstAnalysisQuery>,
) -> Result<Response, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;
    let repo_id = query
        .repo
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_REPO", "`repo` is required"))?;
    let payload =
        load_code_ast_analysis_response(state.as_ref(), path, repo_id, query.line).await?;
    retrieval_chunks_arrow_response(&payload.retrieval_atoms)
        .map_err(|error| StudioApiError::internal("CODE_AST_RETRIEVAL_ARROW_FAILED", error, None))
}
