use std::sync::Arc;

use crate::analyzers::{
    DocCoverageQuery, DocCoverageResult, RegisteredRepository, RepoIntelligenceError,
    RepoOverviewQuery, RepoOverviewResult, RepositoryAnalysisOutput, build_doc_coverage,
    build_repo_overview,
};
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repository, map_repo_intelligence_error,
};

pub(crate) async fn run_repo_overview(
    state: Arc<GatewayState>,
    repo_id: String,
) -> Result<RepoOverviewResult, StudioApiError> {
    let repository = configured_repository(&state.studio, repo_id.as_str())
        .map_err(map_repo_intelligence_error)?;
    if !repository.has_repo_intelligence_plugins() {
        return Ok(build_search_only_repo_overview(&repository));
    }

    run_repo_analysis_summary(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_OVERVIEW_PANIC",
        "Repo overview task failed unexpectedly",
        move |analysis| {
            Ok::<_, RepoIntelligenceError>(build_repo_overview(
                &RepoOverviewQuery { repo_id },
                &analysis,
            ))
        },
    )
    .await
}

fn build_search_only_repo_overview(repository: &RegisteredRepository) -> RepoOverviewResult {
    RepoOverviewResult {
        repo_id: repository.id.clone(),
        display_name: repository.id.clone(),
        revision: None,
        module_count: 0,
        symbol_count: 0,
        example_count: 0,
        doc_count: 0,
        hierarchical_uri: Some(format!("repo://{}", repository.id)),
        hierarchy: Some(vec!["repo".to_string(), repository.id.clone()]),
    }
}

pub(crate) async fn run_repo_doc_coverage(
    state: Arc<GatewayState>,
    repo_id: String,
    module_id: Option<String>,
) -> Result<DocCoverageResult, StudioApiError> {
    run_repo_analysis_summary(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_DOC_COVERAGE_PANIC",
        "Repo doc coverage task failed unexpectedly",
        move |analysis| {
            Ok::<_, RepoIntelligenceError>(build_doc_coverage(
                &DocCoverageQuery { repo_id, module_id },
                &analysis,
            ))
        },
    )
    .await
}

async fn run_repo_analysis_summary<T, F>(
    state: Arc<GatewayState>,
    repo_id: String,
    panic_code: &'static str,
    panic_message: &'static str,
    build: F,
) -> Result<T, StudioApiError>
where
    T: Send + 'static,
    F: FnOnce(RepositoryAnalysisOutput) -> Result<T, RepoIntelligenceError> + Send + 'static,
{
    with_repo_analysis(state, repo_id, panic_code, panic_message, build).await
}

#[cfg(test)]
#[path = "../../../../../../../tests/unit/gateway/studio/router/handlers/repo/analysis/service.rs"]
mod tests;
