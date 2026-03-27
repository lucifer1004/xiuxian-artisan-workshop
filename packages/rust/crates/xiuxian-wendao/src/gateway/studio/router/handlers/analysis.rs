//! Studio API endpoint handlers.

use axum::{
    Json,
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use std::fs;
use std::sync::Arc;

use crate::analyzers::analyze_registered_repository_with_registry;
use crate::gateway::studio::router::code_ast::build_code_ast_analysis_response;
use crate::gateway::studio::router::retrieval_arrow::retrieval_chunks_arrow_response;
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repository, map_repo_intelligence_error,
};
use crate::gateway::studio::types::{CodeAstAnalysisResponse, MarkdownAnalysisResponse};

/// Query parameters for Markdown analysis.
#[derive(Debug, Deserialize)]
pub struct MarkdownAnalysisQuery {
    /// The repository-relative path to the Markdown file.
    pub path: Option<String>,
}

/// Query parameters for Code AST analysis.
#[derive(Debug, Deserialize)]
pub struct CodeAstAnalysisQuery {
    /// The repository-relative path to the source file.
    pub path: Option<String>,
    /// Optional repository identifier.
    pub repo: Option<String>,
    /// Optional 1-based line number for focused analysis.
    pub line: Option<usize>,
}

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

async fn load_markdown_analysis_response(
    state: &GatewayState,
    path: &str,
) -> Result<MarkdownAnalysisResponse, StudioApiError> {
    crate::gateway::studio::analysis::analyze_markdown(state.studio.as_ref(), path)
        .await
        .map_err(|error| {
            StudioApiError::internal("MARKDOWN_ANALYSIS_FAILED", error.to_string(), None)
        })
}

async fn load_code_ast_analysis_response(
    state: &GatewayState,
    path: &str,
    repo_id: &str,
    line_hint: Option<usize>,
) -> Result<CodeAstAnalysisResponse, StudioApiError> {
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(&state.studio, repo_id).map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);

    let repo_id = repo_id.to_string();
    let repo_path = path.to_string();

    tokio::task::spawn_blocking(
        move || -> Result<CodeAstAnalysisResponse, crate::analyzers::RepoIntelligenceError> {
            let analysis = analyze_registered_repository_with_registry(
                &repository,
                cwd.as_path(),
                &plugin_registry,
            )?;
            let source_content = repository
                .path
                .as_ref()
                .map(|root| root.join(&repo_path))
                .filter(|path| path.is_file())
                .and_then(|path| fs::read_to_string(path).ok());
            Ok(build_code_ast_analysis_response(
                repo_id,
                repo_path,
                line_hint,
                source_content.as_deref(),
                &analysis,
            ))
        },
    )
    .await
    .map_err(|error: tokio::task::JoinError| {
        StudioApiError::internal(
            "CODE_AST_PANIC",
            "Code AST analysis task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)
}
