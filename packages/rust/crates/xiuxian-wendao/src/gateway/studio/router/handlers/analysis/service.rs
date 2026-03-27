use std::fs;
use std::sync::Arc;

use crate::analyzers::analyze_registered_repository_with_registry;
use crate::gateway::studio::router::code_ast::build_code_ast_analysis_response;
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repository, map_repo_intelligence_error,
};
use crate::gateway::studio::types::{CodeAstAnalysisResponse, MarkdownAnalysisResponse};

pub(crate) async fn load_markdown_analysis_response(
    state: &GatewayState,
    path: &str,
) -> Result<MarkdownAnalysisResponse, StudioApiError> {
    crate::gateway::studio::analysis::analyze_markdown(state.studio.as_ref(), path)
        .await
        .map_err(|error| {
            StudioApiError::internal("MARKDOWN_ANALYSIS_FAILED", error.to_string(), None)
        })
}

pub(crate) async fn load_code_ast_analysis_response(
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
