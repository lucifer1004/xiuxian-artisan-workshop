use std::sync::Arc;

use arrow::array::{Array, ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

use super::definitions as search_index;
use crate::duckdb::{DataFusionLocalRelationEngine, LocalRelationEngine, LocalRelationEngineKind};
#[cfg(feature = "duckdb")]
use crate::duckdb::{DuckDbLocalRelationEngine, resolve_search_duckdb_runtime};
use crate::search::SearchPlaneStatusSnapshot;
use xiuxian_vector::EngineRecordBatch;

const STATUS_DIAGNOSTICS_TABLE: &str = "status_rollup_rows";

pub(crate) struct SearchIndexDiagnosticsRollup {
    pub(crate) total: usize,
    pub(crate) idle: usize,
    pub(crate) indexing: usize,
    pub(crate) ready: usize,
    pub(crate) degraded: usize,
    pub(crate) failed: usize,
    pub(crate) compaction_pending: usize,
    pub(crate) maintenance_summary: Option<search_index::SearchIndexAggregateMaintenanceSummary>,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn configured_status_diagnostics_engine_kind() -> Result<LocalRelationEngineKind, String>
{
    configured_status_diagnostics_engine().map(|engine| engine.kind())
}

pub(crate) async fn summarize_status_diagnostics_rollup(
    snapshot: &SearchPlaneStatusSnapshot,
) -> Result<SearchIndexDiagnosticsRollup, String> {
    let engine = configured_status_diagnostics_engine()?;
    let (schema, batches) = status_snapshot_relation(snapshot)?;
    engine.register_record_batches(STATUS_DIAGNOSTICS_TABLE, schema, batches)?;
    let batches = engine
        .query_batches(STATUS_DIAGNOSTICS_SQL)
        .await
        .map_err(|error| format!("status diagnostics rollup query failed: {error}"))?;
    decode_status_rollup(batches.as_slice())
}

fn configured_status_diagnostics_engine() -> Result<Box<dyn LocalRelationEngine>, String> {
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

fn status_snapshot_relation(
    snapshot: &SearchPlaneStatusSnapshot,
) -> Result<(SchemaRef, Vec<EngineRecordBatch>), String> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("corpus", DataType::Utf8, false),
        Field::new("phase", DataType::Utf8, false),
        Field::new("prewarm_running", DataType::Boolean, false),
        Field::new("prewarm_queue_depth", DataType::Int64, false),
        Field::new("compaction_running", DataType::Boolean, false),
        Field::new("compaction_queue_depth", DataType::Int64, false),
        Field::new("compaction_queue_aged", DataType::Boolean, false),
        Field::new("compaction_pending", DataType::Boolean, false),
    ]));
    let corpus = snapshot
        .corpora
        .iter()
        .map(|status| status.corpus.as_str())
        .collect::<Vec<_>>();
    let phase = snapshot
        .corpora
        .iter()
        .map(|status| match status.phase {
            crate::search::SearchPlanePhase::Idle => "idle",
            crate::search::SearchPlanePhase::Indexing => "indexing",
            crate::search::SearchPlanePhase::Ready => "ready",
            crate::search::SearchPlanePhase::Degraded => "degraded",
            crate::search::SearchPlanePhase::Failed => "failed",
        })
        .collect::<Vec<_>>();
    let prewarm_running = snapshot
        .corpora
        .iter()
        .map(|status| status.maintenance.prewarm_running)
        .collect::<Vec<_>>();
    let prewarm_queue_depth = snapshot
        .corpora
        .iter()
        .map(|status| i64::from(status.maintenance.prewarm_queue_depth))
        .collect::<Vec<_>>();
    let compaction_running = snapshot
        .corpora
        .iter()
        .map(|status| status.maintenance.compaction_running)
        .collect::<Vec<_>>();
    let compaction_queue_depth = snapshot
        .corpora
        .iter()
        .map(|status| i64::from(status.maintenance.compaction_queue_depth))
        .collect::<Vec<_>>();
    let compaction_queue_aged = snapshot
        .corpora
        .iter()
        .map(|status| status.maintenance.compaction_queue_aged.is_aged())
        .collect::<Vec<_>>();
    let compaction_pending = snapshot
        .corpora
        .iter()
        .map(|status| status.maintenance.compaction_pending)
        .collect::<Vec<_>>();

    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(StringArray::from(corpus)) as ArrayRef,
            Arc::new(StringArray::from(phase)) as ArrayRef,
            Arc::new(BooleanArray::from(prewarm_running)) as ArrayRef,
            Arc::new(Int64Array::from(prewarm_queue_depth)) as ArrayRef,
            Arc::new(BooleanArray::from(compaction_running)) as ArrayRef,
            Arc::new(Int64Array::from(compaction_queue_depth)) as ArrayRef,
            Arc::new(BooleanArray::from(compaction_queue_aged)) as ArrayRef,
            Arc::new(BooleanArray::from(compaction_pending)) as ArrayRef,
        ],
    )
    .map_err(|error| format!("failed to build status diagnostics relation batch: {error}"))?;

    Ok((schema, vec![batch]))
}

fn decode_status_rollup(
    batches: &[EngineRecordBatch],
) -> Result<SearchIndexDiagnosticsRollup, String> {
    let total = decode_rollup_value(batches, "total")?;
    let idle = decode_rollup_value(batches, "idle")?;
    let indexing = decode_rollup_value(batches, "indexing")?;
    let ready = decode_rollup_value(batches, "ready")?;
    let degraded = decode_rollup_value(batches, "degraded")?;
    let failed = decode_rollup_value(batches, "failed")?;
    let compaction_pending = decode_rollup_value(batches, "compaction_pending")?;
    let prewarm_running_count = decode_rollup_value(batches, "prewarm_running_count")?;
    let prewarm_queued_corpus_count = decode_rollup_value(batches, "prewarm_queued_corpus_count")?;
    let max_prewarm_queue_depth = decode_rollup_value(batches, "max_prewarm_queue_depth")?;
    let compaction_running_count = decode_rollup_value(batches, "compaction_running_count")?;
    let compaction_queued_corpus_count =
        decode_rollup_value(batches, "compaction_queued_corpus_count")?;
    let max_compaction_queue_depth = decode_rollup_value(batches, "max_compaction_queue_depth")?;
    let compaction_pending_count = decode_rollup_value(batches, "compaction_pending_count")?;
    let aged_compaction_queue_count = decode_rollup_value(batches, "aged_compaction_queue_count")?;

    let maintenance_summary = if prewarm_running_count == 0
        && prewarm_queued_corpus_count == 0
        && compaction_running_count == 0
        && compaction_queued_corpus_count == 0
        && compaction_pending_count == 0
        && aged_compaction_queue_count == 0
    {
        None
    } else {
        Some(search_index::SearchIndexAggregateMaintenanceSummary {
            prewarm_running_count,
            prewarm_queued_corpus_count,
            max_prewarm_queue_depth: u32::try_from(max_prewarm_queue_depth).unwrap_or(u32::MAX),
            compaction_running_count,
            compaction_queued_corpus_count,
            max_compaction_queue_depth: u32::try_from(max_compaction_queue_depth)
                .unwrap_or(u32::MAX),
            compaction_pending_count,
            aged_compaction_queue_count,
        })
    };

    Ok(SearchIndexDiagnosticsRollup {
        total,
        idle,
        indexing,
        ready,
        degraded,
        failed,
        compaction_pending,
        maintenance_summary,
    })
}

fn decode_rollup_value(batches: &[EngineRecordBatch], column: &str) -> Result<usize, String> {
    let batch = batches
        .iter()
        .find(|batch| batch.num_rows() > 0)
        .ok_or_else(|| format!("status diagnostics query returned no rows for `{column}`"))?;
    let column_index = batch.schema().index_of(column).map_err(|error| {
        format!("missing status diagnostics column `{column}` in rollup batch: {error}")
    })?;
    let values = batch.column(column_index);
    if let Some(values) = values.as_any().downcast_ref::<Int64Array>() {
        let value = values.value(0);
        return usize::try_from(value)
            .map_err(|_| format!("status diagnostics value for `{column}` overflowed usize"));
    }
    Err(format!(
        "unsupported status diagnostics column type for `{column}`: {:?}",
        values.data_type()
    ))
}

const STATUS_DIAGNOSTICS_SQL: &str = r"
SELECT
    CAST(COUNT(*) AS BIGINT) AS total,
    CAST(SUM(CASE WHEN phase = 'idle' THEN 1 ELSE 0 END) AS BIGINT) AS idle,
    CAST(SUM(CASE WHEN phase = 'indexing' THEN 1 ELSE 0 END) AS BIGINT) AS indexing,
    CAST(SUM(CASE WHEN phase = 'ready' THEN 1 ELSE 0 END) AS BIGINT) AS ready,
    CAST(SUM(CASE WHEN phase = 'degraded' THEN 1 ELSE 0 END) AS BIGINT) AS degraded,
    CAST(SUM(CASE WHEN phase = 'failed' THEN 1 ELSE 0 END) AS BIGINT) AS failed,
    CAST(SUM(CASE WHEN compaction_pending THEN 1 ELSE 0 END) AS BIGINT) AS compaction_pending,
    CAST(SUM(CASE WHEN prewarm_running THEN 1 ELSE 0 END) AS BIGINT) AS prewarm_running_count,
    CAST(SUM(CASE WHEN prewarm_queue_depth > 0 THEN 1 ELSE 0 END) AS BIGINT) AS prewarm_queued_corpus_count,
    CAST(COALESCE(MAX(prewarm_queue_depth), 0) AS BIGINT) AS max_prewarm_queue_depth,
    CAST(SUM(CASE WHEN compaction_running THEN 1 ELSE 0 END) AS BIGINT) AS compaction_running_count,
    CAST(SUM(CASE WHEN compaction_queue_depth > 0 THEN 1 ELSE 0 END) AS BIGINT) AS compaction_queued_corpus_count,
    CAST(COALESCE(MAX(compaction_queue_depth), 0) AS BIGINT) AS max_compaction_queue_depth,
    CAST(SUM(CASE WHEN compaction_pending THEN 1 ELSE 0 END) AS BIGINT) AS compaction_pending_count,
    CAST(SUM(CASE WHEN compaction_queue_aged THEN 1 ELSE 0 END) AS BIGINT) AS aged_compaction_queue_count
FROM status_rollup_rows
";
