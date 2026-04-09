use arrow_flight::sql::CommandGetCatalogs;
use tempfile::TempDir;

use crate::search::queries::flightsql::build_studio_flightsql_service;

use super::fixtures::{fetch_command_batches, fixture_service, string_column_values};

#[tokio::test]
async fn flightsql_catalogs_discovery_reports_stable_wendao_catalog() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = build_studio_flightsql_service(fixture_service(&temp_dir));

    let batches = fetch_command_batches(&service, CommandGetCatalogs {}).await;

    assert_eq!(batches.len(), 1);
    assert_eq!(
        string_column_values(&batches[0], "catalog_name"),
        vec!["wendao".to_string()]
    );
}
