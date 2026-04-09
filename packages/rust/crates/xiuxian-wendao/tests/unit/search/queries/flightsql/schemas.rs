use arrow_flight::sql::CommandGetDbSchemas;
use tempfile::TempDir;

use crate::search::queries::flightsql::build_studio_flightsql_service;

use super::fixtures::{
    fetch_command_batches, fixture_service, publish_reference_hits, publish_repo_content_chunks,
    repo_document, sample_hit, string_column_values,
};

#[tokio::test]
async fn flightsql_schema_discovery_derives_names_from_registered_sql_scope() {
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
        CommandGetDbSchemas {
            catalog: Some("wendao".to_string()),
            db_schema_filter_pattern: None,
        },
    )
    .await;

    assert_eq!(batches.len(), 1);
    assert_eq!(
        string_column_values(&batches[0], "db_schema_name"),
        vec![
            "local".to_string(),
            "repo".to_string(),
            "system".to_string(),
        ]
    );
}
