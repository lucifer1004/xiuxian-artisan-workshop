#[cfg(feature = "duckdb")]
use crate::duckdb::LocalRelationEngineKind;
use crate::search::SearchCorpusKind;
use crate::search::queries::SearchQueryService;
use crate::search::queries::sql::execute_sql_query;
#[cfg(feature = "duckdb")]
use crate::search::queries::sql::execution::service::SqlQueryExecutionRoute;
#[cfg(feature = "duckdb")]
use crate::search::queries::sql::execution::service::execute_sql_query_with_route;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
    STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
#[cfg(feature = "duckdb")]
use crate::search::queries::sql::tests::fixtures::write_search_duckdb_runtime_override;
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, publish_reference_hits, sample_hit,
};
#[cfg(feature = "duckdb")]
use crate::search::queries::sql::tests::fixtures::{
    publish_local_symbol_hits, sample_local_symbol_hit,
};

#[tokio::test]
async fn sql_query_execution_returns_transport_neutral_payload() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let hits = vec![
        sample_hit("AlphaService", "src/lib.rs", 10),
        sample_hit("BetaThing", "src/beta.rs", 20),
    ];
    publish_reference_hits(&service, "sql-execution-1", &hits).await;
    let query_service = SearchQueryService::new(service.clone());

    let result = execute_sql_query(
        &query_service,
        format!(
            "SELECT name, path FROM {} WHERE name = 'AlphaService'",
            SearchCorpusKind::ReferenceOccurrence
        )
        .as_str(),
    )
    .await
    .unwrap_or_else(|error| panic!("execute shared SQL query: {error}"));
    let payload = result
        .payload()
        .unwrap_or_else(|error| panic!("shared SQL payload: {error}"));

    assert_eq!(
        payload.metadata.catalog_table_name,
        STUDIO_SQL_CATALOG_TABLE_NAME
    );
    assert_eq!(
        payload.metadata.column_catalog_table_name,
        STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME
    );
    assert_eq!(
        payload.metadata.view_source_catalog_table_name,
        STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME
    );
    assert_eq!(payload.metadata.result_batch_count, 1);
    assert_eq!(payload.metadata.result_row_count, 1);
    assert_eq!(payload.metadata.registered_input_bytes, None);
    assert_eq!(payload.metadata.result_bytes, None);
    assert_eq!(payload.metadata.local_relation_materialization_state, None);
    assert_eq!(payload.metadata.local_temp_storage_peak_bytes, None);
    assert_eq!(payload.metadata.local_relation_engine, None);
    assert_eq!(payload.metadata.duckdb_registration_strategy, None);
    assert_eq!(payload.metadata.registered_input_batch_count, None);
    assert_eq!(payload.metadata.registered_input_row_count, None);
    assert_eq!(payload.metadata.registration_time_ms, None);
    assert_eq!(payload.metadata.local_query_execution_time_ms, None);
    assert_eq!(payload.batches.len(), 1);
    assert_eq!(payload.batches[0].row_count, 1);
    assert_eq!(
        payload.batches[0].rows[0]
            .get("name")
            .unwrap_or_else(|| panic!("shared SQL payload row should include `name`")),
        "AlphaService"
    );
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn sql_query_execution_routes_reference_occurrence_through_local_parquet_duckdb() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let service = fixture_service(&temp_dir);
    publish_reference_hits(
        &service,
        "sql-execution-2",
        &[sample_hit("AlphaService", "src/lib.rs", 10)],
    )
    .await;
    let query_service = SearchQueryService::new(service.clone());

    let (route, result) = execute_sql_query_with_route(
        &query_service,
        "SELECT name, path FROM reference_occurrence ORDER BY name",
    )
    .await
    .unwrap_or_else(|error| panic!("execute shared SQL routed query: {error}"));
    let payload = result
        .payload()
        .unwrap_or_else(|error| panic!("shared SQL routed payload: {error}"));

    assert!(matches!(
        route,
        SqlQueryExecutionRoute::LocalParquet {
            corpus: SearchCorpusKind::ReferenceOccurrence,
            ref table_name,
            engine_kind: LocalRelationEngineKind::DuckDb,
        } if table_name == "reference_occurrence"
    ));
    assert_eq!(
        payload.metadata.catalog_table_name,
        STUDIO_SQL_CATALOG_TABLE_NAME
    );
    assert_eq!(
        payload.metadata.column_catalog_table_name,
        STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME
    );
    assert_eq!(
        payload.metadata.view_source_catalog_table_name,
        STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME
    );
    assert_eq!(
        payload.metadata.registered_tables,
        vec![
            SearchCorpusKind::ReferenceOccurrence.to_string(),
            STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME.to_string(),
            STUDIO_SQL_CATALOG_TABLE_NAME.to_string(),
            STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME.to_string(),
        ]
    );
    assert_eq!(payload.metadata.registered_table_count, 4);
    assert_eq!(payload.metadata.registered_view_count, 0);
    assert_eq!(payload.metadata.registered_column_count, 33);
    assert_eq!(payload.metadata.registered_view_source_count, 0);
    assert_eq!(payload.metadata.result_batch_count, 1);
    assert_eq!(payload.metadata.result_row_count, 1);
    assert!(payload.metadata.supports_information_schema);
    assert_eq!(payload.batches.len(), 1);
    assert_eq!(
        payload.batches[0].rows[0]
            .get("name")
            .unwrap_or_else(|| panic!("shared SQL routed payload row should include `name`")),
        "AlphaService"
    );
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn sql_query_execution_routes_catalog_queries_through_shared_sql_duckdb() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let service = fixture_service(&temp_dir);
    publish_reference_hits(
        &service,
        "sql-execution-3",
        &[sample_hit("AlphaService", "src/lib.rs", 10)],
    )
    .await;
    let query_service = SearchQueryService::new(service);

    let (route, result) = execute_sql_query_with_route(
        &query_service,
        format!(
            "SELECT sql_table_name FROM {STUDIO_SQL_CATALOG_TABLE_NAME} ORDER BY sql_table_name LIMIT 1"
        )
        .as_str(),
    )
    .await
    .unwrap_or_else(|error| panic!("execute shared SQL catalog query: {error}"));
    let payload = result
        .payload()
        .unwrap_or_else(|error| panic!("shared SQL catalog payload: {error}"));

    assert_eq!(
        route,
        SqlQueryExecutionRoute::SharedSql {
            engine_kind: LocalRelationEngineKind::DuckDb,
        }
    );
    assert_eq!(
        payload.metadata.catalog_table_name,
        STUDIO_SQL_CATALOG_TABLE_NAME
    );
    assert_eq!(payload.metadata.result_batch_count, 1);
    assert!(payload.metadata.result_row_count >= 1);
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn sql_query_execution_routes_local_symbol_logical_view_through_shared_sql_duckdb() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let service = fixture_service(&temp_dir);
    publish_local_symbol_hits(
        &service,
        "sql-execution-4",
        &[sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10)],
    )
    .await;
    let query_service = SearchQueryService::new(service);

    let (route, result) = execute_sql_query_with_route(
        &query_service,
        "SELECT name, path FROM local_symbol ORDER BY name",
    )
    .await
    .unwrap_or_else(|error| panic!("execute shared SQL local-symbol logical view query: {error}"));
    let payload = result
        .payload()
        .unwrap_or_else(|error| panic!("shared SQL local-symbol payload: {error}"));

    assert_eq!(
        route,
        SqlQueryExecutionRoute::SharedSql {
            engine_kind: LocalRelationEngineKind::DuckDb,
        }
    );
    assert_eq!(payload.metadata.result_batch_count, 1);
    assert_eq!(payload.metadata.result_row_count, 1);
    assert_eq!(
        payload.batches[0].rows[0]
            .get("name")
            .unwrap_or_else(|| panic!("shared SQL local-symbol payload row should include `name`")),
        "AlphaSymbol"
    );
    assert_eq!(
        payload.batches[0].rows[0]
            .get("path")
            .unwrap_or_else(|| panic!("shared SQL local-symbol payload row should include `path`")),
        "src/lib.rs"
    );
}
