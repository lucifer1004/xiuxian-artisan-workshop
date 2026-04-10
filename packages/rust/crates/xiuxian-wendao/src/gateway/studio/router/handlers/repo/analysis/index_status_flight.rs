use std::sync::Arc;

use arrow::array::{Array, ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use async_trait::async_trait;
use serde::Serialize;
use xiuxian_vector_store::{
    LanceDataType, LanceField, LanceInt32Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RepoIndexStatusFlightRouteProvider,
};

#[cfg(all(test, feature = "duckdb"))]
use crate::duckdb::LocalRelationEngineKind;
use crate::duckdb::{
    DataFusionLocalRelationEngine, LocalRelationEngine, LocalRelationRegistrationHint,
};
use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::handlers::repo::command_service::run_repo_index_status;
use crate::repo_index::{RepoIndexPhase, RepoIndexStatusResponse};
#[cfg(feature = "duckdb")]
use crate::{duckdb::DuckDbLocalRelationEngine, duckdb::resolve_search_duckdb_runtime};

const REPO_INDEX_STATUS_DIAGNOSTICS_TABLE: &str = "repo_index_status_rows";
const REPO_INDEX_STATUS_SUMMARY_SQL: &str = r#"
SELECT
  CAST(COUNT(*) AS BIGINT) AS total,
  CAST(COALESCE(SUM(CASE WHEN is_active THEN 1 ELSE 0 END), 0) AS BIGINT) AS active,
  CAST(COALESCE(SUM(CASE WHEN phase = 'queued' THEN 1 ELSE 0 END), 0) AS BIGINT) AS queued,
  CAST(COALESCE(SUM(CASE WHEN phase = 'checking' THEN 1 ELSE 0 END), 0) AS BIGINT) AS checking,
  CAST(COALESCE(SUM(CASE WHEN phase = 'syncing' THEN 1 ELSE 0 END), 0) AS BIGINT) AS syncing,
  CAST(COALESCE(SUM(CASE WHEN phase = 'indexing' THEN 1 ELSE 0 END), 0) AS BIGINT) AS indexing,
  CAST(COALESCE(SUM(CASE WHEN phase = 'ready' THEN 1 ELSE 0 END), 0) AS BIGINT) AS ready,
  CAST(COALESCE(SUM(CASE WHEN phase = 'unsupported' THEN 1 ELSE 0 END), 0) AS BIGINT) AS unsupported,
  CAST(COALESCE(SUM(CASE WHEN phase = 'failed' THEN 1 ELSE 0 END), 0) AS BIGINT) AS failed
FROM repo_index_status_rows
"#;
const REPO_INDEX_STATUS_ACTIVE_IDS_SQL: &str = r#"
SELECT repo_id
FROM repo_index_status_rows
WHERE is_active
ORDER BY active_order ASC, repo_id ASC
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoIndexStatusDiagnosticsSummary {
    pub(crate) total: usize,
    pub(crate) active: usize,
    pub(crate) queued: usize,
    pub(crate) checking: usize,
    pub(crate) syncing: usize,
    pub(crate) indexing: usize,
    pub(crate) ready: usize,
    pub(crate) unsupported: usize,
    pub(crate) failed: usize,
    pub(crate) current_repo_id: Option<String>,
    pub(crate) active_repo_ids: Vec<String>,
}

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
        let response = repo_index_status_response_with_diagnostics(&response).await;
        let batch = build_repo_index_status_flight_batch(&response)?;
        let metadata = build_repo_index_status_flight_metadata(&response)?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

#[cfg(all(test, feature = "duckdb"))]
pub(crate) fn configured_repo_index_status_diagnostics_engine_kind()
-> Result<LocalRelationEngineKind, String> {
    configured_repo_index_status_diagnostics_engine().map(|engine| engine.kind())
}

pub(crate) async fn repo_index_status_response_with_diagnostics(
    response: &RepoIndexStatusResponse,
) -> RepoIndexStatusResponse {
    let summary = match summarize_repo_index_status_diagnostics(response).await {
        Ok(summary) => summary,
        Err(_) => return response.clone(),
    };
    RepoIndexStatusResponse {
        total: summary.total,
        active: summary.active,
        queued: summary.queued,
        checking: summary.checking,
        syncing: summary.syncing,
        indexing: summary.indexing,
        ready: summary.ready,
        unsupported: summary.unsupported,
        failed: summary.failed,
        target_concurrency: response.target_concurrency,
        max_concurrency: response.max_concurrency,
        sync_concurrency_limit: response.sync_concurrency_limit,
        current_repo_id: summary.current_repo_id,
        active_repo_ids: summary.active_repo_ids,
        repos: response.repos.clone(),
    }
}

fn configured_repo_index_status_diagnostics_engine() -> Result<Box<dyn LocalRelationEngine>, String>
{
    #[cfg(feature = "duckdb")]
    {
        let runtime = resolve_search_duckdb_runtime();
        if runtime.enabled {
            return DuckDbLocalRelationEngine::from_runtime(runtime)
                .map(|engine| Box::new(engine) as Box<dyn LocalRelationEngine>);
        }
    }

    Ok(Box::new(
        DataFusionLocalRelationEngine::new_with_information_schema(),
    ))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn summarize_repo_index_status_diagnostics(
    response: &RepoIndexStatusResponse,
) -> Result<RepoIndexStatusDiagnosticsSummary, String> {
    let engine = configured_repo_index_status_diagnostics_engine()?;
    let (schema, batches) = repo_index_status_relation(response)?;
    engine.register_record_batches_with_hint(
        REPO_INDEX_STATUS_DIAGNOSTICS_TABLE,
        schema,
        batches,
        LocalRelationRegistrationHint::RepeatedUse,
    )?;
    let summary_batches = engine
        .query_batches(REPO_INDEX_STATUS_SUMMARY_SQL)
        .await
        .map_err(|error| format!("repo index status diagnostics summary query failed: {error}"))?;
    let active_id_batches = engine
        .query_batches(REPO_INDEX_STATUS_ACTIVE_IDS_SQL)
        .await
        .map_err(|error| {
            format!("repo index status diagnostics active identity query failed: {error}")
        })?;
    decode_repo_index_status_summary(summary_batches.as_slice(), active_id_batches.as_slice())
}

fn repo_index_status_relation(
    response: &RepoIndexStatusResponse,
) -> Result<(SchemaRef, Vec<RecordBatch>), String> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("repo_id", DataType::Utf8, false),
        Field::new("phase", DataType::Utf8, false),
        Field::new("is_active", DataType::Boolean, false),
        Field::new("active_order", DataType::Int64, true),
        Field::new("attempt_count", DataType::Int64, false),
    ]));
    let active_repo_ids = response
        .active_repo_ids
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let active_order = response
        .active_repo_ids
        .iter()
        .enumerate()
        .map(|(index, repo_id)| {
            (
                repo_id.as_str(),
                i64::try_from(index).unwrap_or(i64::MAX).saturating_add(1),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let repo_ids = response
        .repos
        .iter()
        .map(|status| status.repo_id.as_str())
        .collect::<Vec<_>>();
    let phases = response
        .repos
        .iter()
        .map(|status| repo_index_phase_label(status.phase))
        .collect::<Vec<_>>();
    let is_active = response
        .repos
        .iter()
        .map(|status| active_repo_ids.contains(status.repo_id.as_str()))
        .collect::<Vec<_>>();
    let active_order = response
        .repos
        .iter()
        .map(|status| active_order.get(status.repo_id.as_str()).copied())
        .collect::<Vec<_>>();
    let attempt_counts = response
        .repos
        .iter()
        .map(|status| i64::try_from(status.attempt_count).unwrap_or(i64::MAX))
        .collect::<Vec<_>>();

    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(StringArray::from(repo_ids)) as ArrayRef,
            Arc::new(StringArray::from(phases)) as ArrayRef,
            Arc::new(BooleanArray::from(is_active)) as ArrayRef,
            Arc::new(Int64Array::from(active_order)) as ArrayRef,
            Arc::new(Int64Array::from(attempt_counts)) as ArrayRef,
        ],
    )
    .map_err(|error| {
        format!("failed to build repo index status diagnostics relation batch: {error}")
    })?;
    Ok((schema, vec![batch]))
}

fn repo_index_phase_label(phase: RepoIndexPhase) -> &'static str {
    match phase {
        RepoIndexPhase::Idle => "idle",
        RepoIndexPhase::Queued => "queued",
        RepoIndexPhase::Checking => "checking",
        RepoIndexPhase::Syncing => "syncing",
        RepoIndexPhase::Indexing => "indexing",
        RepoIndexPhase::Ready => "ready",
        RepoIndexPhase::Unsupported => "unsupported",
        RepoIndexPhase::Failed => "failed",
    }
}

fn decode_repo_index_status_summary(
    batches: &[RecordBatch],
    active_id_batches: &[RecordBatch],
) -> Result<RepoIndexStatusDiagnosticsSummary, String> {
    let batch = batches.first().ok_or_else(|| {
        "repo index status diagnostics summary query returned no batches".to_string()
    })?;
    let active_repo_ids = decode_active_repo_ids(active_id_batches)?;
    if batch.num_rows() == 0 {
        return Ok(RepoIndexStatusDiagnosticsSummary {
            total: 0,
            active: 0,
            queued: 0,
            checking: 0,
            syncing: 0,
            indexing: 0,
            ready: 0,
            unsupported: 0,
            failed: 0,
            current_repo_id: None,
            active_repo_ids,
        });
    }
    let current_repo_id = active_repo_ids.first().cloned();
    Ok(RepoIndexStatusDiagnosticsSummary {
        total: decode_usize_summary_column(batch, "total")?,
        active: decode_usize_summary_column(batch, "active")?,
        queued: decode_usize_summary_column(batch, "queued")?,
        checking: decode_usize_summary_column(batch, "checking")?,
        syncing: decode_usize_summary_column(batch, "syncing")?,
        indexing: decode_usize_summary_column(batch, "indexing")?,
        ready: decode_usize_summary_column(batch, "ready")?,
        unsupported: decode_usize_summary_column(batch, "unsupported")?,
        failed: decode_usize_summary_column(batch, "failed")?,
        current_repo_id,
        active_repo_ids,
    })
}

fn decode_active_repo_ids(batches: &[RecordBatch]) -> Result<Vec<String>, String> {
    let mut active_repo_ids = Vec::new();
    for batch in batches {
        let Some(column) = batch.column_by_name("repo_id") else {
            return Err(
                "repo index status diagnostics active identity rows missing `repo_id`".to_string(),
            );
        };
        let values = column
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| {
                "repo index status diagnostics active identity `repo_id` must be Utf8".to_string()
            })?;
        for row in 0..batch.num_rows() {
            active_repo_ids.push(values.value(row).to_string());
        }
    }
    Ok(active_repo_ids)
}

fn decode_usize_summary_column(batch: &RecordBatch, column_name: &str) -> Result<usize, String> {
    let column = batch
        .column_by_name(column_name)
        .ok_or_else(|| format!("repo index status diagnostics summary missing `{column_name}`"))?;
    let values = column
        .as_any()
        .downcast_ref::<Int64Array>()
        .ok_or_else(|| {
            format!("repo index status diagnostics summary `{column_name}` must be Int64")
        })?;
    usize::try_from(values.value(0)).map_err(|error| {
        format!("repo index status diagnostics summary `{column_name}` overflowed usize: {error}")
    })
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
