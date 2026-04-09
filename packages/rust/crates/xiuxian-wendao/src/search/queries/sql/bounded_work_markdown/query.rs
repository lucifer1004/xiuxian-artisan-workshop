use std::path::Path;

use super::super::execution::{
    SqlQueryMetadata, SqlQueryPayload, sql_query_payload_from_engine_batches,
};
use super::register::{
    BOUNDED_WORK_MARKDOWN_TABLE_NAME, bootstrap_bounded_work_markdown_query_engine,
};

/// Execute one SQL query over the bounded-work `markdown` surface.
///
/// # Errors
///
/// Returns an error when the bounded-work markdown table cannot be registered
/// or when the SQL statement cannot be executed or serialized into the stable
/// payload shape.
pub async fn query_bounded_work_markdown_payload(
    root: &Path,
    query_text: &str,
) -> Result<SqlQueryPayload, String> {
    let (query_engine, rows) = bootstrap_bounded_work_markdown_query_engine(root)?;
    let engine_batches = query_engine
        .sql_batches(query_text)
        .await
        .map_err(|error| {
            format!("bounded-work markdown SQL query execution failed for `{query_text}`: {error}")
        })?;
    let result_row_count = engine_batches
        .iter()
        .map(xiuxian_vector::EngineRecordBatch::num_rows)
        .sum();
    let metadata = SqlQueryMetadata {
        catalog_table_name: BOUNDED_WORK_MARKDOWN_TABLE_NAME.to_string(),
        column_catalog_table_name: String::new(),
        view_source_catalog_table_name: String::new(),
        supports_information_schema: true,
        registered_tables: vec![BOUNDED_WORK_MARKDOWN_TABLE_NAME.to_string()],
        registered_table_count: 1,
        registered_view_count: 0,
        registered_column_count: 7,
        registered_view_source_count: 0,
        result_batch_count: engine_batches.len(),
        result_row_count,
    };

    let _ = rows;
    sql_query_payload_from_engine_batches(metadata, &engine_batches)
}
