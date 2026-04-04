use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceInt32Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RepoOverviewFlightRouteProvider,
};

use crate::analyzers::RepoOverviewResult;
use crate::gateway::studio::router::handlers::repo::analysis::service::run_repo_overview;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

#[derive(Clone)]
pub(crate) struct StudioRepoOverviewFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioRepoOverviewFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioRepoOverviewFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRepoOverviewFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RepoOverviewFlightRouteProvider for StudioRepoOverviewFlightRouteProvider {
    async fn repo_overview_batch(
        &self,
        repo_id: &str,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        let response = run_repo_overview(Arc::clone(&self.state), repo_id.to_string())
            .await
            .map_err(|error| map_studio_api_error(&error))?;
        let batch = build_repo_overview_flight_batch(&response)?;
        let metadata = build_repo_overview_flight_metadata(&response)?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

fn build_repo_overview_flight_batch(
    response: &RepoOverviewResult,
) -> Result<LanceRecordBatch, String> {
    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("repoId", LanceDataType::Utf8, false),
            LanceField::new("displayName", LanceDataType::Utf8, false),
            LanceField::new("revision", LanceDataType::Utf8, true),
            LanceField::new("moduleCount", LanceDataType::Int32, false),
            LanceField::new("symbolCount", LanceDataType::Int32, false),
            LanceField::new("exampleCount", LanceDataType::Int32, false),
            LanceField::new("docCount", LanceDataType::Int32, false),
            LanceField::new("hierarchicalUri", LanceDataType::Utf8, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec![response.repo_id.clone()])),
            Arc::new(LanceStringArray::from(vec![response.display_name.clone()])),
            Arc::new(LanceStringArray::from(vec![response.revision.clone()])),
            Arc::new(LanceInt32Array::from(vec![
                i32::try_from(response.module_count).map_err(|error| {
                    format!("failed to encode repo overview module_count: {error}")
                })?,
            ])),
            Arc::new(LanceInt32Array::from(vec![
                i32::try_from(response.symbol_count).map_err(|error| {
                    format!("failed to encode repo overview symbol_count: {error}")
                })?,
            ])),
            Arc::new(LanceInt32Array::from(vec![
                i32::try_from(response.example_count).map_err(|error| {
                    format!("failed to encode repo overview example_count: {error}")
                })?,
            ])),
            Arc::new(LanceInt32Array::from(vec![
                i32::try_from(response.doc_count).map_err(|error| {
                    format!("failed to encode repo overview doc_count: {error}")
                })?,
            ])),
            Arc::new(LanceStringArray::from(vec![
                response.hierarchical_uri.clone(),
            ])),
        ],
    )
    .map_err(|error| error.to_string())
}

fn build_repo_overview_flight_metadata(response: &RepoOverviewResult) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "repoId": response.repo_id,
        "displayName": response.display_name,
        "revision": response.revision,
        "moduleCount": response.module_count,
        "symbolCount": response.symbol_count,
        "exampleCount": response.example_count,
        "docCount": response.doc_count,
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
    fn repo_overview_flight_batch_preserves_summary_fields() {
        let batch = build_repo_overview_flight_batch(&RepoOverviewResult {
            repo_id: "gateway-sync".to_string(),
            display_name: "Gateway Sync".to_string(),
            revision: Some("rev:123".to_string()),
            module_count: 3,
            symbol_count: 8,
            example_count: 2,
            doc_count: 5,
            hierarchical_uri: Some("repo://gateway-sync".to_string()),
            hierarchy: Some(vec!["repo".to_string(), "gateway-sync".to_string()]),
        })
        .expect("repo overview batch should build");

        assert_eq!(batch.num_rows(), 1);
        let display_name = batch
            .column_by_name("displayName")
            .expect("displayName column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("displayName should be utf8");
        assert_eq!(display_name.value(0), "Gateway Sync");

        let doc_count = batch
            .column_by_name("docCount")
            .expect("docCount column")
            .as_any()
            .downcast_ref::<LanceInt32Array>()
            .expect("docCount should be int32");
        assert_eq!(doc_count.value(0), 5);
    }

    #[test]
    fn repo_overview_flight_metadata_preserves_summary_fields() {
        let metadata = build_repo_overview_flight_metadata(&RepoOverviewResult {
            repo_id: "gateway-sync".to_string(),
            display_name: "Gateway Sync".to_string(),
            revision: Some("rev:123".to_string()),
            module_count: 3,
            symbol_count: 8,
            example_count: 2,
            doc_count: 5,
            hierarchical_uri: Some("repo://gateway-sync".to_string()),
            hierarchy: Some(vec!["repo".to_string(), "gateway-sync".to_string()]),
        })
        .expect("repo overview metadata should encode");

        let payload: serde_json::Value =
            serde_json::from_slice(&metadata).expect("metadata should decode");
        assert_eq!(payload["repoId"], "gateway-sync");
        assert_eq!(payload["displayName"], "Gateway Sync");
        assert_eq!(payload["revision"], "rev:123");
        assert_eq!(payload["moduleCount"], 3);
        assert_eq!(payload["symbolCount"], 8);
        assert_eq!(payload["exampleCount"], 2);
        assert_eq!(payload["docCount"], 5);
        assert_eq!(payload["hierarchicalUri"], "repo://gateway-sync");
        assert_eq!(payload["hierarchy"][0], "repo");
    }
}
