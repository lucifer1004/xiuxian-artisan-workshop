use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::LanceRecordBatch;
use xiuxian_wendao_runtime::transport::{
    CodeAstAnalysisFlightRouteProvider, MarkdownAnalysisFlightRouteProvider,
};

use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::router::handlers::analysis::service::{
    load_code_ast_analysis_response, load_markdown_analysis_response,
};
use crate::gateway::studio::router::retrieval_arrow::build_retrieval_chunks_flight_batch;

#[derive(Clone)]
pub(crate) struct StudioMarkdownAnalysisFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioMarkdownAnalysisFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioMarkdownAnalysisFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioMarkdownAnalysisFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl MarkdownAnalysisFlightRouteProvider for StudioMarkdownAnalysisFlightRouteProvider {
    async fn markdown_analysis_batch(&self, path: &str) -> Result<LanceRecordBatch, String> {
        let response = load_markdown_analysis_response(self.state.as_ref(), path)
            .await
            .map_err(|error| map_studio_api_error(error))?;
        build_retrieval_chunks_flight_batch(response.retrieval_atoms.as_slice())
    }
}

#[derive(Clone)]
pub(crate) struct StudioCodeAstAnalysisFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioCodeAstAnalysisFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioCodeAstAnalysisFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioCodeAstAnalysisFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl CodeAstAnalysisFlightRouteProvider for StudioCodeAstAnalysisFlightRouteProvider {
    async fn code_ast_analysis_batch(
        &self,
        path: &str,
        repo_id: &str,
        line_hint: Option<usize>,
    ) -> Result<LanceRecordBatch, String> {
        let response =
            load_code_ast_analysis_response(self.state.as_ref(), path, repo_id, line_hint)
                .await
                .map_err(|error| map_studio_api_error(error))?;
        build_retrieval_chunks_flight_batch(response.retrieval_atoms.as_slice())
    }
}

fn map_studio_api_error(error: StudioApiError) -> String {
    error
        .error
        .details
        .clone()
        .unwrap_or_else(|| format!("{}: {}", error.code(), error.error.message))
}
