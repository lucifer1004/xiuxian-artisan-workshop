use crate::search::queries::SearchQueryService;
use crate::search::queries::sql::execute_sql_query;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
    STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, publish_reference_hits, sample_hit,
};
use crate::search_plane::SearchCorpusKind;

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
    assert_eq!(payload.batches.len(), 1);
    assert_eq!(payload.batches[0].row_count, 1);
    assert_eq!(
        payload.batches[0].rows[0]
            .get("name")
            .unwrap_or_else(|| panic!("shared SQL payload row should include `name`")),
        "AlphaService"
    );
}
