use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceInt32Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RepoIndexStatusFlightRouteProvider,
};

use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::handlers::repo::command_service::run_repo_index_status;
use crate::repo_index::RepoIndexStatusResponse;

#[derive(Clone)]
pub(crate) struct StudioRepoIndexStatusFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioRepoIndexStatusFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioRepoIndexStatusFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRepoIndexStatusFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RepoIndexStatusFlightRouteProvider for StudioRepoIndexStatusFlightRouteProvider {
    async fn repo_index_status_batch(
        &self,
        repo_id: Option<&str>,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        let response = run_repo_index_status(&self.state, repo_id);
        let batch = build_repo_index_status_flight_batch(&response)?;
        let metadata = build_repo_index_status_flight_metadata(&response)?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

pub(crate) fn build_repo_index_status_flight_batch(
    response: &RepoIndexStatusResponse,
) -> Result<LanceRecordBatch, String> {
    let repos_json = serde_json::to_string(&response.repos).map_err(|error| error.to_string())?;
    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("total", LanceDataType::Int32, false),
            LanceField::new("queued", LanceDataType::Int32, false),
            LanceField::new("checking", LanceDataType::Int32, false),
            LanceField::new("syncing", LanceDataType::Int32, false),
            LanceField::new("indexing", LanceDataType::Int32, false),
            LanceField::new("ready", LanceDataType::Int32, false),
            LanceField::new("unsupported", LanceDataType::Int32, false),
            LanceField::new("failed", LanceDataType::Int32, false),
            LanceField::new("targetConcurrency", LanceDataType::Int32, false),
            LanceField::new("maxConcurrency", LanceDataType::Int32, false),
            LanceField::new("syncConcurrencyLimit", LanceDataType::Int32, false),
            LanceField::new("currentRepoId", LanceDataType::Utf8, true),
            LanceField::new("reposJson", LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.total,
                "total",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.queued,
                "queued",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.checking,
                "checking",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.syncing,
                "syncing",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.indexing,
                "indexing",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.ready,
                "ready",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.unsupported,
                "unsupported",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.failed,
                "failed",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.target_concurrency,
                "target_concurrency",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.max_concurrency,
                "max_concurrency",
            )?])),
            Arc::new(LanceInt32Array::from(vec![encode_i32(
                response.sync_concurrency_limit,
                "sync_concurrency_limit",
            )?])),
            Arc::new(LanceStringArray::from(vec![
                response.current_repo_id.clone(),
            ])),
            Arc::new(LanceStringArray::from(vec![repos_json])),
        ],
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn build_repo_index_status_flight_metadata(
    response: &RepoIndexStatusResponse,
) -> Result<Vec<u8>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct RepoIndexStatusFlightMetadata<'a> {
        total: usize,
        queued: usize,
        checking: usize,
        syncing: usize,
        indexing: usize,
        ready: usize,
        unsupported: usize,
        failed: usize,
        target_concurrency: usize,
        max_concurrency: usize,
        sync_concurrency_limit: usize,
        current_repo_id: Option<String>,
        repos: &'a [crate::repo_index::RepoIndexEntryStatus],
    }

    serde_json::to_vec(&RepoIndexStatusFlightMetadata {
        total: response.total,
        queued: response.queued,
        checking: response.checking,
        syncing: response.syncing,
        indexing: response.indexing,
        ready: response.ready,
        unsupported: response.unsupported,
        failed: response.failed,
        target_concurrency: response.target_concurrency,
        max_concurrency: response.max_concurrency,
        sync_concurrency_limit: response.sync_concurrency_limit,
        current_repo_id: response.current_repo_id.clone(),
        repos: &response.repos,
    })
    .map_err(|error| error.to_string())
}

fn encode_i32(value: usize, field: &str) -> Result<i32, String> {
    i32::try_from(value)
        .map_err(|error| format!("failed to encode repo index status {field} as int32: {error}"))
}

#[cfg(test)]
#[path = "../../../../../../../tests/unit/gateway/studio/router/handlers/repo/analysis/index_status_flight.rs"]
mod tests;
