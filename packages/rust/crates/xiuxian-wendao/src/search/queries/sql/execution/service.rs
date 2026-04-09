use crate::search::queries::SearchQueryService;

use super::result::{SqlQueryMetadata, SqlQueryPayload, SqlQueryResult};
use crate::search::queries::sql::registration::SqlQuerySurface;

pub(crate) async fn execute_sql_query(
    service: &SearchQueryService,
    query_text: &str,
) -> Result<SqlQueryResult, String> {
    let query_core = service.open_core().await?;
    let metadata = sql_query_metadata(query_core.surface());
    let engine_batches = query_core
        .engine()
        .sql_batches(query_text)
        .await
        .map_err(|error| {
            format!("shared SQL query execution failed for `{query_text}`: {error}")
        })?;
    let result_row_count = engine_batches
        .iter()
        .map(xiuxian_vector::EngineRecordBatch::num_rows)
        .sum();
    let result_batch_count = engine_batches.len();
    let mut metadata = metadata;
    metadata.result_batch_count = result_batch_count;
    metadata.result_row_count = result_row_count;
    Ok(SqlQueryResult::new(metadata, engine_batches))
}

fn sql_query_metadata(query_surface: &SqlQuerySurface) -> SqlQueryMetadata {
    SqlQueryMetadata {
        catalog_table_name: query_surface.catalog_table_name.clone(),
        column_catalog_table_name: query_surface.column_catalog_table_name.clone(),
        view_source_catalog_table_name: query_surface.view_source_catalog_table_name.clone(),
        supports_information_schema: true,
        registered_tables: query_surface.registered_table_names(),
        registered_table_count: query_surface.registered_table_count(),
        registered_view_count: query_surface.registered_view_count(),
        registered_column_count: query_surface.registered_column_count(),
        registered_view_source_count: query_surface.registered_view_source_count(),
        result_batch_count: 0,
        result_row_count: 0,
        registered_input_bytes: None,
        result_bytes: None,
        local_relation_materialization_state: None,
        local_temp_storage_peak_bytes: None,
        local_relation_engine: None,
        duckdb_registration_strategy: None,
        registered_input_batch_count: None,
        registered_input_row_count: None,
        registration_time_ms: None,
        local_query_execution_time_ms: None,
    }
}

/// Execute one request-scoped SQL query and serialize the result payload.
///
/// # Errors
///
/// Returns an error when the request-scoped SQL surface cannot be registered
/// or when the SQL statement cannot be executed or serialized into the stable
/// CLI-friendly payload shape.
pub async fn query_sql_payload(
    service: &SearchQueryService,
    query_text: &str,
) -> Result<SqlQueryPayload, String> {
    execute_sql_query(service, query_text).await?.payload()
}
