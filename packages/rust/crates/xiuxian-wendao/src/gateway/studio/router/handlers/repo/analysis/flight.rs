use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::{LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RepoDocCoverageFlightRouteProvider,
};

use crate::analyzers::{DocCoverageResult, DocRecord};
use crate::gateway::studio::router::handlers::repo::analysis::service::run_repo_doc_coverage;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

#[derive(Clone)]
pub(crate) struct StudioRepoDocCoverageFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioRepoDocCoverageFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioRepoDocCoverageFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRepoDocCoverageFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RepoDocCoverageFlightRouteProvider for StudioRepoDocCoverageFlightRouteProvider {
    async fn repo_doc_coverage_batch(
        &self,
        repo_id: &str,
        module_id: Option<&str>,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        let response = run_repo_doc_coverage(
            Arc::clone(&self.state),
            repo_id.to_string(),
            module_id.map(ToString::to_string),
        )
        .await
        .map_err(|error| map_studio_api_error(&error))?;
        let batch = build_repo_doc_coverage_flight_batch(response.docs.as_slice())?;
        let metadata = build_repo_doc_coverage_flight_metadata(&response)?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

fn build_repo_doc_coverage_flight_batch(docs: &[DocRecord]) -> Result<LanceRecordBatch, String> {
    let repo_ids = docs
        .iter()
        .map(|doc| doc.repo_id.clone())
        .collect::<Vec<_>>();
    let doc_ids = docs
        .iter()
        .map(|doc| doc.doc_id.clone())
        .collect::<Vec<_>>();
    let titles = docs.iter().map(|doc| doc.title.clone()).collect::<Vec<_>>();
    let paths = docs.iter().map(|doc| doc.path.clone()).collect::<Vec<_>>();
    let formats = docs
        .iter()
        .map(|doc| doc.format.clone())
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("repoId", LanceDataType::Utf8, false),
            LanceField::new("docId", LanceDataType::Utf8, false),
            LanceField::new("title", LanceDataType::Utf8, false),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("format", LanceDataType::Utf8, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(repo_ids)),
            Arc::new(LanceStringArray::from(doc_ids)),
            Arc::new(LanceStringArray::from(titles)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(formats)),
        ],
    )
    .map_err(|error| error.to_string())
}

fn build_repo_doc_coverage_flight_metadata(
    response: &DocCoverageResult,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "repoId": response.repo_id,
        "moduleId": response.module_id,
        "coveredSymbols": response.covered_symbols,
        "uncoveredSymbols": response.uncovered_symbols,
        "hierarchicalUri": response.hierarchical_uri,
        "hierarchy": response.hierarchy,
    }))
    .map_err(|error| error.to_string())
}

fn map_studio_api_error(error: &StudioApiError) -> String {
    error
        .error
        .details
        .clone()
        .unwrap_or_else(|| format!("{}: {}", error.code(), error.error.message))
}

#[cfg(test)]
#[path = "../../../../../../../tests/unit/gateway/studio/router/handlers/repo/analysis/flight.rs"]
mod tests;
