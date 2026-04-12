use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Stable metadata returned for one request-scoped SQL query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SqlQueryMetadata {
    /// Stable table name for the request-scoped table inventory catalog.
    pub catalog_table_name: String,
    /// Stable table name for the request-scoped column inventory catalog.
    pub column_catalog_table_name: String,
    /// Stable table name for the request-scoped logical-view source catalog.
    pub view_source_catalog_table_name: String,
    /// Whether the SQL engine exposes `information_schema`.
    pub supports_information_schema: bool,
    /// Stable SQL-visible object names registered for the current request.
    pub registered_tables: Vec<String>,
    /// Count of registered SQL-visible tables for the current request.
    pub registered_table_count: usize,
    /// Count of registered logical views for the current request.
    pub registered_view_count: usize,
    /// Count of registered SQL-visible columns for the current request.
    pub registered_column_count: usize,
    /// Count of logical-view source rows for the current request.
    pub registered_view_source_count: usize,
    /// Count of result batches returned by the query.
    pub result_batch_count: usize,
    /// Count of rows returned across all result batches.
    pub result_row_count: usize,
    /// Count of array-backed bytes registered into the bounded local relation
    /// before query execution when the caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_input_bytes: Option<u64>,
    /// Count of array-backed bytes returned across all result batches when the
    /// caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_bytes: Option<u64>,
    /// Stable bounded local relation materialization-state label when the
    /// caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_relation_materialization_state: Option<String>,
    /// Peak temp-storage bytes observed for the last bounded local query when
    /// the caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_temp_storage_peak_bytes: Option<u64>,
    /// Stable local relation-engine label for bounded local analytics when the
    /// caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_relation_engine: Option<String>,
    /// Stable `DuckDB` registration-strategy label when the bounded local engine
    /// exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duckdb_registration_strategy: Option<String>,
    /// Count of input batches registered into the bounded local relation before
    /// query execution when the caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_input_batch_count: Option<usize>,
    /// Count of rows registered into the bounded local relation before query
    /// execution when the caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_input_row_count: Option<usize>,
    /// Milliseconds spent registering the bounded local relation before query
    /// execution when the caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_time_ms: Option<u64>,
    /// Milliseconds spent executing the bounded local SQL statement when the
    /// caller exposes that detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_query_execution_time_ms: Option<u64>,
}

/// Stable description of one SQL result column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SqlColumnPayload {
    /// Column name as exposed to the caller.
    pub name: String,
    /// Stable `Arrow` or `DataFusion` data-type label.
    pub data_type: String,
    /// Whether the column accepts null values.
    pub nullable: bool,
}

/// Stable JSON-friendly representation of one SQL result batch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SqlBatchPayload {
    /// Row count for this batch.
    pub row_count: usize,
    /// Ordered column descriptors for this batch schema.
    pub columns: Vec<SqlColumnPayload>,
    /// Ordered row payloads for this batch.
    pub rows: Vec<Map<String, Value>>,
}

/// Stable JSON-friendly representation of one SQL query result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SqlQueryPayload {
    /// Request-scoped discovery and result metadata.
    pub metadata: SqlQueryMetadata,
    /// Materialized result batches.
    pub batches: Vec<SqlBatchPayload>,
}
