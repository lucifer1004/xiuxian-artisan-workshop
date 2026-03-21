//! Studio API endpoint handlers.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::analyzers::analyze_registered_repository_with_registry;
use crate::gateway::studio::router::code_ast::build_code_ast_analysis_response;
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
pub async fn markdown(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<MarkdownAnalysisQuery>,
) -> Result<Json<MarkdownAnalysisResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;

    let response = crate::gateway::studio::analysis::analyze_markdown(state.studio.as_ref(), path)
        .await
        .map_err(|error| {
            StudioApiError::internal("MARKDOWN_ANALYSIS_FAILED", error.to_string(), None)
        })?;

    Ok(Json(response))
}

/// Analyzes code file AST and projections.
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

    let line_hint = query.line;
    let cwd = state.studio.project_root.clone();
    let repository =
        configured_repository(&state.studio, repo_id).map_err(map_repo_intelligence_error)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);

    let repo_id = repo_id.to_string();
    let repo_path = path.to_string();

    let payload = tokio::task::spawn_blocking(
        move || -> Result<CodeAstAnalysisResponse, crate::analyzers::RepoIntelligenceError> {
            let analysis = analyze_registered_repository_with_registry(
                &repository,
                cwd.as_path(),
                &plugin_registry,
            )?;
            Ok(build_code_ast_analysis_response(
                repo_id, repo_path, line_hint, &analysis,
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
    .map_err(map_repo_intelligence_error)?;

    Ok(Json(payload))
}
