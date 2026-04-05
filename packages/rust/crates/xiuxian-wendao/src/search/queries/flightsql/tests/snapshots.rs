use arrow_flight::sql::{
    CommandGetCatalogs, CommandGetSqlInfo, CommandGetTables, CommandStatementQuery, SqlInfo,
};
use serde_json::json;
use tempfile::TempDir;

use crate::search::queries::flightsql::build_studio_flightsql_service;
use crate::search::queries::tests::snapshots::assert_query_json_snapshot;

use super::fixtures::{
    fetch_command_batches, fixture_service, flight_batches_snapshot, publish_reference_hits,
    publish_repo_content_chunks, repo_document, sample_hit,
};

#[tokio::test]
async fn flightsql_query_adapter_snapshots_surface_payloads() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "flightsql-snapshot-1",
        &[sample_hit("AlphaService", "src/alpha.rs", 11)],
    )
    .await;
    publish_repo_content_chunks(
        &search_plane,
        "alpha/repo",
        &[repo_document(
            "src/lib.rs",
            "pub fn alpha() -> usize { 1 }\n",
            "rust",
            1,
        )],
        "rev-1",
    )
    .await;
    let service = build_studio_flightsql_service(search_plane);
    let statement = CommandStatementQuery {
        query: "SELECT name, path FROM reference_occurrence ORDER BY name".to_string(),
        transaction_id: None,
    };
    let catalogs = CommandGetCatalogs {};
    let tables = CommandGetTables {
        catalog: Some("wendao".to_string()),
        db_schema_filter_pattern: None,
        table_name_filter_pattern: Some("%".to_string()),
        table_types: Vec::new(),
        include_schema: false,
    };
    let sql_info = CommandGetSqlInfo {
        info: vec![
            SqlInfo::FlightSqlServerName as u32,
            SqlInfo::FlightSqlServerVersion as u32,
        ],
    };

    let payload = json!({
        "statement": {
            "query": statement.query,
            "batches": flight_batches_snapshot(&fetch_command_batches(&service, statement).await),
        },
        "catalogs": flight_batches_snapshot(&fetch_command_batches(&service, catalogs).await),
        "tables": flight_batches_snapshot(&fetch_command_batches(&service, tables).await),
        "sql_info": flight_batches_snapshot(&fetch_command_batches(&service, sql_info).await),
    });

    assert_query_json_snapshot("flightsql_query_surface_payload", payload);
}
