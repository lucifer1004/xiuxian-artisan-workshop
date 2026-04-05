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
mod tests {
    use xiuxian_vector::LanceArray;

    use super::*;

    #[test]
    fn repo_doc_coverage_flight_batch_preserves_doc_rows() {
        let batch = build_repo_doc_coverage_flight_batch(&[
            DocRecord {
                repo_id: "gateway-sync".to_string(),
                doc_id: "repo:gateway-sync:doc:README.md".to_string(),
                title: "README".to_string(),
                path: "README.md".to_string(),
                format: Some("markdown".to_string()),
            },
            DocRecord {
                repo_id: "gateway-sync".to_string(),
                doc_id: "repo:gateway-sync:doc:docs/solve.md".to_string(),
                title: "solve".to_string(),
                path: "docs/solve.md".to_string(),
                format: None,
            },
        ])
        .unwrap_or_else(|error| panic!("repo doc coverage batch should build: {error}"));

        assert_eq!(batch.num_rows(), 2);
        let Some(doc_id_column) = batch.column_by_name("docId") else {
            panic!("docId column");
        };
        let Some(doc_ids) = doc_id_column.as_any().downcast_ref::<LanceStringArray>() else {
            panic!("docId should be utf8");
        };
        assert_eq!(doc_ids.value(0), "repo:gateway-sync:doc:README.md");
        assert_eq!(doc_ids.value(1), "repo:gateway-sync:doc:docs/solve.md");

        let Some(format_column) = batch.column_by_name("format") else {
            panic!("format column");
        };
        let Some(formats) = format_column.as_any().downcast_ref::<LanceStringArray>() else {
            panic!("format should be utf8");
        };
        assert_eq!(formats.value(0), "markdown");
        assert!(formats.is_null(1));
    }

    #[test]
    fn repo_doc_coverage_flight_metadata_preserves_summary_fields() {
        let metadata = build_repo_doc_coverage_flight_metadata(&DocCoverageResult {
            repo_id: "gateway-sync".to_string(),
            module_id: Some("GatewaySyncPkg".to_string()),
            docs: Vec::new(),
            covered_symbols: 3,
            uncovered_symbols: 1,
            hierarchical_uri: Some("repo://gateway-sync/docs".to_string()),
            hierarchy: Some(vec!["repo".to_string(), "gateway-sync".to_string()]),
        })
        .unwrap_or_else(|error| panic!("repo doc coverage metadata should encode: {error}"));

        let payload: serde_json::Value = serde_json::from_slice(&metadata)
            .unwrap_or_else(|error| panic!("metadata should decode: {error}"));
        assert_eq!(payload["repoId"], "gateway-sync");
        assert_eq!(payload["moduleId"], "GatewaySyncPkg");
        assert_eq!(payload["coveredSymbols"], 3);
        assert_eq!(payload["uncoveredSymbols"], 1);
        assert_eq!(payload["hierarchicalUri"], "repo://gateway-sync/docs");
        assert_eq!(payload["hierarchy"][0], "repo");
    }
}
