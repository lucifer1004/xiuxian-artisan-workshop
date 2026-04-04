use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceInt32Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RepoIndexStatusFlightRouteProvider,
};

use crate::gateway::studio::repo_index::RepoIndexStatusResponse;
use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::handlers::repo::command_service::run_repo_index_status;

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
        repos: &'a [crate::gateway::studio::repo_index::RepoIndexEntryStatus],
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
mod tests {
    use xiuxian_vector::LanceArray;

    use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};

    use super::*;

    #[test]
    fn repo_index_status_flight_batch_preserves_summary_fields() {
        let batch = build_repo_index_status_flight_batch(&RepoIndexStatusResponse {
            total: 3,
            active: 2,
            queued: 1,
            checking: 0,
            syncing: 1,
            indexing: 1,
            ready: 1,
            unsupported: 0,
            failed: 0,
            target_concurrency: 2,
            max_concurrency: 4,
            sync_concurrency_limit: 1,
            current_repo_id: Some("gateway-sync".to_string()),
            active_repo_ids: vec!["gateway-sync".to_string()],
            repos: vec![RepoIndexEntryStatus {
                repo_id: "gateway-sync".to_string(),
                phase: RepoIndexPhase::Ready,
                queue_position: None,
                last_error: None,
                last_revision: Some("rev:123".to_string()),
                updated_at: Some("2026-04-03T19:15:00Z".to_string()),
                attempt_count: 2,
            }],
        })
        .expect("repo index status batch should build");

        assert_eq!(batch.num_rows(), 1);
        let ready = batch
            .column_by_name("ready")
            .expect("ready column")
            .as_any()
            .downcast_ref::<LanceInt32Array>()
            .expect("ready should be int32");
        assert_eq!(ready.value(0), 1);

        let repos_json = batch
            .column_by_name("reposJson")
            .expect("reposJson column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("reposJson should be utf8");
        assert!(repos_json.value(0).contains("gateway-sync"));
    }

    #[test]
    fn repo_index_status_flight_metadata_preserves_summary_fields() {
        let metadata = build_repo_index_status_flight_metadata(&RepoIndexStatusResponse {
            total: 3,
            active: 2,
            queued: 1,
            checking: 0,
            syncing: 1,
            indexing: 1,
            ready: 1,
            unsupported: 0,
            failed: 0,
            target_concurrency: 2,
            max_concurrency: 4,
            sync_concurrency_limit: 1,
            current_repo_id: Some("gateway-sync".to_string()),
            active_repo_ids: vec!["gateway-sync".to_string()],
            repos: vec![RepoIndexEntryStatus {
                repo_id: "gateway-sync".to_string(),
                phase: RepoIndexPhase::Ready,
                queue_position: None,
                last_error: None,
                last_revision: Some("rev:123".to_string()),
                updated_at: Some("2026-04-03T19:15:00Z".to_string()),
                attempt_count: 2,
            }],
        })
        .expect("repo index status metadata should encode");

        let payload: serde_json::Value =
            serde_json::from_slice(&metadata).expect("metadata should decode");
        assert_eq!(payload["total"], 3);
        assert_eq!(payload["syncConcurrencyLimit"], 1);
        assert_eq!(payload["currentRepoId"], "gateway-sync");
        assert_eq!(payload["repos"][0]["repoId"], "gateway-sync");
    }
}
