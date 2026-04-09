use xiuxian_wendao_runtime::transport::SqlFlightRouteProvider;

use crate::search::queries::sql::provider::StudioSqlFlightRouteProvider;
use crate::search::queries::sql::provider::metadata::StudioSqlFlightMetadata;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
    STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, publish_reference_hits, sample_hit,
};
use crate::search::{SearchCorpusKind, SearchPlaneService};

#[tokio::test]
async fn studio_sql_flight_provider_queries_registered_reference_occurrence_table() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let hits = vec![
        sample_hit("AlphaService", "src/lib.rs", 10),
        sample_hit("BetaThing", "src/beta.rs", 20),
    ];
    let epoch = publish_reference_hits(&service, "fp-sql-1", &hits).await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let _engine_table_name = SearchPlaneService::local_epoch_engine_table_name(
        SearchCorpusKind::ReferenceOccurrence,
        epoch,
    );
    let sql_table_name = SearchCorpusKind::ReferenceOccurrence.to_string();
    let response = provider
        .sql_query_batches(
            format!("SELECT name, path FROM {sql_table_name} WHERE name = 'AlphaService'").as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("SQL query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert!(
        response.batches[0].column_by_name("name").is_some(),
        "name column should exist"
    );
    assert!(
        response.batches[0].column_by_name("path").is_some(),
        "path column should exist"
    );

    let app_metadata: StudioSqlFlightMetadata =
        serde_json::from_slice(response.app_metadata.as_slice())
            .unwrap_or_else(|error| panic!("decode app metadata: {error}"));
    assert_eq!(
        app_metadata,
        StudioSqlFlightMetadata {
            catalog_table_name: STUDIO_SQL_CATALOG_TABLE_NAME.to_string(),
            column_catalog_table_name: STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME.to_string(),
            view_source_catalog_table_name: STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME.to_string(),
            supports_information_schema: true,
            registered_tables: vec![
                sql_table_name,
                STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME.to_string(),
                STUDIO_SQL_CATALOG_TABLE_NAME.to_string(),
                STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME.to_string(),
            ],
            registered_table_count: 4,
            registered_view_count: 0,
            registered_column_count: 33,
            registered_view_source_count: 0,
            result_batch_count: 1,
            result_row_count: 1,
        }
    );
}
