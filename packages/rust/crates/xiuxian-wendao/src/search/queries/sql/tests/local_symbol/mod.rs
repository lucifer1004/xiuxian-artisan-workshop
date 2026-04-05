use xiuxian_wendao_runtime::transport::SqlFlightRouteProvider;

use crate::search::queries::sql::provider::StudioSqlFlightRouteProvider;
use crate::search::queries::sql::provider::metadata::StudioSqlFlightMetadata;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, nullable_string_column_values, publish_local_symbol_hits,
    sample_local_symbol_hit, string_column_values, u64_column_values,
};
use crate::search_plane::SearchCorpusKind;

#[tokio::test]
async fn studio_sql_flight_provider_queries_local_symbol_logical_view() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_local_symbol_hits(
        &service,
        "fp-local-symbol-sql-1",
        &[
            sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10),
            sample_local_symbol_hit("BetaSymbol", "src/beta.rs", 20),
        ],
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::LocalSymbol.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT name, path FROM {logical_view_name} WHERE name = 'AlphaSymbol' ORDER BY path"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("local symbol logical view query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "name"),
        vec!["AlphaSymbol".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "path"),
        vec!["src/lib.rs".to_string()]
    );

    let app_metadata: StudioSqlFlightMetadata =
        serde_json::from_slice(response.app_metadata.as_slice())
            .unwrap_or_else(|error| panic!("decode app metadata: {error}"));
    assert_eq!(app_metadata.registered_view_count, 1);
    assert_eq!(app_metadata.registered_view_source_count, 1);
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_local_symbol_logical_view_in_tables_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_local_symbol_hits(
        &service,
        "fp-local-symbol-sql-2",
        &[sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10)],
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::LocalSymbol.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_table_name, corpus, scope, sql_object_kind, source_count, repo_id FROM {STUDIO_SQL_CATALOG_TABLE_NAME} WHERE sql_table_name = '{logical_view_name}'"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("local symbol tables catalog query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_table_name"),
        vec![logical_view_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "corpus"),
        vec![SearchCorpusKind::LocalSymbol.to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "scope"),
        vec!["local_logical".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "sql_object_kind"),
        vec!["view".to_string()]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_count"),
        vec![1]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "repo_id"),
        vec![None]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_local_symbol_view_sources_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_local_symbol_hits(
        &service,
        "fp-local-symbol-sql-3",
        &[sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10)],
    )
    .await;

    let active_epoch = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol)
        .active_epoch
        .unwrap_or_else(|| panic!("active local symbol epoch"));
    let source_table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    let Some(source_table_name) = source_table_names.first().cloned() else {
        panic!("local symbol source table");
    };

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::LocalSymbol.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_view_name, source_sql_table_name, corpus, repo_id, source_ordinal FROM {STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME} WHERE sql_view_name = '{logical_view_name}'"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("local symbol view-source query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_view_name"),
        vec![logical_view_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "source_sql_table_name"),
        vec![source_table_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "corpus"),
        vec![SearchCorpusKind::LocalSymbol.to_string()]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "repo_id"),
        vec![None]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_ordinal"),
        vec![1]
    );
}
