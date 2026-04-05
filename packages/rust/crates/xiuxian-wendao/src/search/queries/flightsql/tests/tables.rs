use arrow_flight::sql::CommandGetTables;
use tempfile::TempDir;

use crate::search::queries::flightsql::build_studio_flightsql_service;

use super::fixtures::{
    fetch_command_batches, fixture_service, publish_reference_hits, publish_repo_content_chunks,
    repo_document, sample_hit, string_value,
};

#[tokio::test]
async fn flightsql_tables_discovery_reports_local_repo_and_system_sql_objects() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "build-1",
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

    let batches = fetch_command_batches(
        &service,
        CommandGetTables {
            catalog: Some("wendao".to_string()),
            db_schema_filter_pattern: None,
            table_name_filter_pattern: Some("%".to_string()),
            table_types: Vec::new(),
            include_schema: false,
        },
    )
    .await;

    let batch = &batches[0];
    let rows = (0..batch.num_rows())
        .map(|row_index| {
            (
                string_value(batch, "db_schema_name", row_index),
                string_value(batch, "table_name", row_index),
                string_value(batch, "table_type", row_index),
            )
        })
        .collect::<Vec<_>>();

    assert!(rows.contains(&(
        "local".to_string(),
        "reference_occurrence".to_string(),
        "TABLE".to_string(),
    )));
    assert!(rows.contains(&(
        "repo".to_string(),
        "repo_content_chunk".to_string(),
        "VIEW".to_string(),
    )));
    assert!(rows.contains(&(
        "system".to_string(),
        "wendao_sql_tables".to_string(),
        "SYSTEM TABLE".to_string(),
    )));
}

#[tokio::test]
async fn flightsql_tables_discovery_can_include_arrow_schema_bytes() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "build-1",
        &[sample_hit("AlphaService", "src/alpha.rs", 11)],
    )
    .await;
    let service = build_studio_flightsql_service(search_plane);

    let batches = fetch_command_batches(
        &service,
        CommandGetTables {
            catalog: Some("wendao".to_string()),
            db_schema_filter_pattern: Some("local".to_string()),
            table_name_filter_pattern: Some("reference_occurrence".to_string()),
            table_types: vec!["TABLE".to_string()],
            include_schema: true,
        },
    )
    .await;

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 1);
    assert_eq!(
        string_value(&batches[0], "table_name", 0),
        "reference_occurrence"
    );
    let table_schema = batches[0]
        .column_by_name("table_schema")
        .unwrap_or_else(|| {
            panic!("table_schema column should be present when include_schema=true")
        });
    assert!(!table_schema.is_null(0));
}
