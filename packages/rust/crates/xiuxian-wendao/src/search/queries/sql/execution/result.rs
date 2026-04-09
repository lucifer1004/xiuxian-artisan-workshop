use arrow::array::{Array, BooleanArray, Float32Array, Float64Array, Int32Array, Int64Array};
use arrow::datatypes::DataType;
use arrow::util::display::array_value_to_string;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use xiuxian_vector::EngineRecordBatch;

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

#[derive(Debug)]
pub(crate) struct SqlQueryResult {
    metadata: SqlQueryMetadata,
    batches: Vec<EngineRecordBatch>,
}

impl SqlQueryResult {
    pub(crate) fn new(metadata: SqlQueryMetadata, batches: Vec<EngineRecordBatch>) -> Self {
        Self { metadata, batches }
    }

    pub(crate) fn into_parts(self) -> (SqlQueryMetadata, Vec<EngineRecordBatch>) {
        (self.metadata, self.batches)
    }

    pub(crate) fn payload(&self) -> Result<SqlQueryPayload, String> {
        sql_query_payload_from_engine_batches(self.metadata.clone(), &self.batches)
    }
}

pub(crate) fn sql_query_payload_from_engine_batches(
    metadata: SqlQueryMetadata,
    batches: &[EngineRecordBatch],
) -> Result<SqlQueryPayload, String> {
    let batches = batches
        .iter()
        .map(batch_payload)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SqlQueryPayload { metadata, batches })
}

pub(crate) fn engine_batches_rows_payload(
    batches: &[EngineRecordBatch],
) -> Result<Vec<Map<String, Value>>, String> {
    batches.iter().try_fold(Vec::new(), |mut rows, batch| {
        rows.extend(engine_batch_rows_payload(batch)?);
        Ok(rows)
    })
}

pub(crate) fn engine_batch_rows_payload(
    batch: &EngineRecordBatch,
) -> Result<Vec<Map<String, Value>>, String> {
    let schema = batch.schema();
    (0..batch.num_rows())
        .map(|row_index| {
            let mut row = Map::new();
            for field in schema.fields() {
                let Some(column) = batch.column_by_name(field.name()) else {
                    return Err(format!(
                        "shared SQL query payload is missing result column `{}`",
                        field.name()
                    ));
                };
                row.insert(
                    field.name().clone(),
                    column_json_value(column.as_ref(), row_index),
                );
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>, _>>()
}

fn batch_payload(batch: &EngineRecordBatch) -> Result<SqlBatchPayload, String> {
    let schema = batch.schema();
    let columns = schema
        .fields()
        .iter()
        .map(|field| SqlColumnPayload {
            name: field.name().clone(),
            data_type: field.data_type().to_string(),
            nullable: field.is_nullable(),
        })
        .collect::<Vec<_>>();
    let rows = engine_batch_rows_payload(batch)?;

    Ok(SqlBatchPayload {
        row_count: batch.num_rows(),
        columns,
        rows,
    })
}

fn column_json_value(column: &dyn Array, index: usize) -> Value {
    if column.is_null(index) {
        return Value::Null;
    }

    match column.data_type() {
        DataType::Boolean => column.as_any().downcast_ref::<BooleanArray>().map_or_else(
            || fallback_column_json_value(column, index),
            |values| Value::Bool(values.value(index)),
        ),
        DataType::UInt64 => Value::Number(Number::from(
            column
                .as_any()
                .downcast_ref::<arrow::array::UInt64Array>()
                .unwrap_or_else(|| panic!("uint64 query payload decode"))
                .value(index),
        )),
        DataType::UInt32 => Value::Number(Number::from(
            column
                .as_any()
                .downcast_ref::<arrow::array::UInt32Array>()
                .unwrap_or_else(|| panic!("uint32 query payload decode"))
                .value(index),
        )),
        DataType::Int64 => Value::Number(Number::from(
            column
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap_or_else(|| panic!("int64 query payload decode"))
                .value(index),
        )),
        DataType::Int32 => Value::Number(Number::from(
            column
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap_or_else(|| panic!("int32 query payload decode"))
                .value(index),
        )),
        DataType::Float64 => Number::from_f64(
            column
                .as_any()
                .downcast_ref::<Float64Array>()
                .unwrap_or_else(|| panic!("float64 query payload decode"))
                .value(index),
        )
        .map_or_else(|| fallback_column_json_value(column, index), Value::Number),
        DataType::Float32 => Number::from_f64(f64::from(
            column
                .as_any()
                .downcast_ref::<Float32Array>()
                .unwrap_or_else(|| panic!("float32 query payload decode"))
                .value(index),
        ))
        .map_or_else(|| fallback_column_json_value(column, index), Value::Number),
        _ => fallback_column_json_value(column, index),
    }
}

fn fallback_column_json_value(column: &dyn Array, index: usize) -> Value {
    Value::String(
        array_value_to_string(column, index)
            .unwrap_or_else(|error| panic!("query payload value decode failed: {error}")),
    )
}
