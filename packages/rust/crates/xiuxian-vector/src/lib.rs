//! xiuxian-vector - High-Performance Embedded Vector Database using `LanceDB`

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use anyhow::Result;
use dashmap::DashMap;
use lance::dataset::Dataset;
use tokio::sync::RwLock;

use ops::DatasetCache;
use ops::DatasetCacheConfig;

// ============================================================================
// Re-exports from xiuxian-lance
// ============================================================================

pub use arrow::record_batch::RecordBatch as EngineRecordBatch;
pub use arrow_codec::{
    attach_record_batch_metadata, attach_record_batch_trace_id, decode_record_batches_ipc,
    encode_record_batch_ipc, encode_record_batches_ipc,
};
pub use lance::deps::arrow_array::ListArray as LanceListArray;
pub use lance::deps::arrow_array::builder::{
    ListBuilder as LanceListBuilder, StringBuilder as LanceStringBuilder,
};
pub use lance::deps::arrow_array::{
    Array as LanceArray, ArrayRef as LanceArrayRef, BooleanArray as LanceBooleanArray,
    Float64Array as LanceFloat64Array, Int32Array as LanceInt32Array,
    RecordBatch as LanceRecordBatch, StringArray as LanceStringArray,
    UInt32Array as LanceUInt32Array, UInt64Array as LanceUInt64Array,
};
pub use lance::deps::arrow_schema::{
    DataType as LanceDataType, Field as LanceField, Schema as LanceSchema,
};
pub use xiuxian_lance::{
    CATEGORY_COLUMN, CONTENT_COLUMN, DEFAULT_DIMENSION, FILE_PATH_COLUMN, ID_COLUMN,
    INTENTS_COLUMN, METADATA_COLUMN, ROUTING_KEYWORDS_COLUMN, SKILL_NAME_COLUMN, THREAD_ID_COLUMN,
    TOOL_NAME_COLUMN, VECTOR_COLUMN, VectorRecordBatchReader, extract_optional_string,
    extract_string,
};

// ============================================================================
// Re-exports from xiuxian-skills (Skills and Knowledge types)
// ============================================================================

pub use xiuxian_skills::skills::{
    ResourceRecord, ResourceScanner, SkillMetadata as XiuxianSkillMetadata, SkillScanner,
    ToolAnnotations, ToolRecord as XiuxianToolRecord, ToolRecord, ToolsScanner,
};

// ============================================================================
// Module Declarations
// ============================================================================

pub use error::VectorStoreError;
pub use keyword::{
    HybridSearchResult, KEYWORD_WEIGHT, KeywordIndex, KeywordSearchBackend, RRF_K, SEMANTIC_WEIGHT,
    apply_rrf, apply_weighted_rrf, distance_to_score, rrf_term, rrf_term_batch,
};
pub use ops::{
    AgenticSearchConfig, ColumnarScanOptions, CompactionStats, FragmentInfo, IndexBuildProgress,
    IndexStats, IndexStatus, IndexThresholds, MergeInsertStats, MigrateResult, MigrationItem,
    QueryIntent, Recommendation, TableColumnAlteration, TableColumnType, TableHealthReport,
    TableInfo, TableNewColumn, TableVersionInfo, XIUXIAN_SCHEMA_VERSION,
    schema_version_from_schema, string_contains_mask,
};
pub use query_support::{
    RETRIEVAL_BEST_SECTION_COLUMN, RETRIEVAL_DOC_TYPE_COLUMN, RETRIEVAL_ID_COLUMN,
    RETRIEVAL_LANGUAGE_COLUMN, RETRIEVAL_LINE_COLUMN, RETRIEVAL_MATCH_REASON_COLUMN,
    RETRIEVAL_PATH_COLUMN, RETRIEVAL_REPO_COLUMN, RETRIEVAL_SCORE_COLUMN, RETRIEVAL_SNIPPET_COLUMN,
    RETRIEVAL_SOURCE_COLUMN, RETRIEVAL_TITLE_COLUMN, RetrievalRow, payload_fetch_record_batch,
    retrieval_result_columns, retrieval_result_schema, retrieval_rows_from_record_batch,
    retrieval_rows_to_record_batch,
};
pub use search::SearchOptions;
pub use search_engine::{
    SearchEngineContext, SearchEnginePartitionColumn, engine_batch_to_lance_batch,
    engine_batches_to_lance_batches, lance_batch_to_engine_batch, lance_batches_to_engine_batches,
    write_engine_batches_to_parquet_file, write_lance_batches_to_parquet_file,
};
pub use search_impl::json_to_lance_where;
pub use skill::{ToolSearchOptions, ToolSearchRequest, ToolSearchResult};

// ============================================================================
// Module Declarations
// ============================================================================

pub mod batch;
pub mod error;
pub mod index;
pub mod keyword;
pub mod ops;
/// Arrow-native retrieval batch helpers used by Wendao query-core adapters.
pub mod query_support;
pub mod search;
pub mod search_cache;
pub mod search_engine;
pub mod skill;
pub mod test_support;

mod arrow_codec;
#[path = "search/search_impl/mod.rs"]
mod search_impl;

// ============================================================================
// Vector Store Core
// ============================================================================

/// Per-table query metrics (in-process; not persisted). Used by [`crate::ops::observability::get_query_metrics`].
pub type QueryMetricsCell = Arc<(AtomicU64, AtomicU64)>; // (query_count, last_query_ms; 0 means None)

/// Callback for index build progress (Started / Progress / Done). Set optionally for polling or UI.
pub type IndexProgressCallback = Arc<dyn Fn(crate::ops::IndexBuildProgress) + Send + Sync>;

/// High-performance embedded vector database using `LanceDB`.
#[derive(Clone)]
pub struct VectorStore {
    base_path: PathBuf,
    datasets: Arc<RwLock<DatasetCache>>,
    dimension: usize,
    /// Optional keyword index used for hybrid dense+keyword retrieval.
    pub keyword_index: Option<Arc<KeywordIndex>>,
    /// Active keyword backend strategy.
    pub keyword_backend: KeywordSearchBackend,
    /// Optional index cache size in bytes. When set, datasets are opened via `DatasetBuilder`.
    pub index_cache_size_bytes: Option<usize>,
    /// In-process per-table query metrics (`query_count`, `last_query_ms`). Wired when `agentic_search` runs.
    pub(crate) query_metrics: Arc<DashMap<String, QueryMetricsCell>>,
    /// Optional callback for index build progress (Started/Done; Progress when Lance exposes API).
    pub(crate) index_progress_callback: Option<IndexProgressCallback>,
    /// When `base_path` is ":memory:", a unique id so each store uses its own temp subdir (avoids `DatasetAlreadyExists`).
    pub(crate) memory_mode_id: Option<u64>,
}

// ----------------------------------------------------------------------------
// Vector Store Implementations (Included via include!)
// ----------------------------------------------------------------------------

include!("ops/core.rs");
include!("ops/writer_impl.rs");
include!("ops/admin_impl.rs");
include!("skill/ops_impl.rs");

impl VectorStore {
    /// Check if a metadata value matches the filter conditions.
    #[must_use]
    pub fn matches_filter(metadata: &serde_json::Value, conditions: &serde_json::Value) -> bool {
        match conditions {
            serde_json::Value::Object(obj) => {
                for (key, value) in obj {
                    let meta_value = if key.contains('.') {
                        let parts: Vec<&str> = key.split('.').collect();
                        let mut current = metadata.clone();
                        for part in parts {
                            if let serde_json::Value::Object(map) = current {
                                current = map.get(part).cloned().unwrap_or(serde_json::Value::Null);
                            } else {
                                return false;
                            }
                        }
                        Some(current)
                    } else {
                        metadata.get(key).cloned()
                    };

                    if let Some(meta_val) = meta_value {
                        match (&meta_val, value) {
                            (serde_json::Value::String(mv), serde_json::Value::String(v)) => {
                                if mv != v {
                                    return false;
                                }
                            }
                            (serde_json::Value::Number(mv), serde_json::Value::Number(v)) => {
                                if mv != v {
                                    return false;
                                }
                            }
                            (serde_json::Value::Bool(mv), serde_json::Value::Bool(v)) => {
                                if mv != v {
                                    return false;
                                }
                            }
                            _ => {
                                let meta_str = meta_val.to_string().trim_matches('"').to_string();
                                let value_str = value.to_string().trim_matches('"').to_string();
                                if meta_str != value_str {
                                    return false;
                                }
                            }
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }
}
