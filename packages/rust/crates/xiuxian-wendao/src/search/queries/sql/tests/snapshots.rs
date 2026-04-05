use serde_json::json;
use xiuxian_wendao_runtime::transport::SqlFlightRouteProvider;

use crate::search::queries::sql::provider::StudioSqlFlightRouteProvider;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
    STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, publish_local_symbol_hits, publish_reference_hits,
    publish_repo_content_chunks, publish_repo_entities, repo_document, sample_hit,
    sample_local_symbol_hit, sql_response_snapshot,
};
use crate::search::queries::tests::snapshots::assert_query_json_snapshot;
use crate::search_plane::SearchCorpusKind;

#[tokio::test]
async fn sql_flight_provider_snapshots_query_surface() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_reference_hits(
        &service,
        "fp-sql-snapshot-reference-1",
        &[
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ],
    )
    .await;
    publish_local_symbol_hits(
        &service,
        "fp-sql-snapshot-local-symbol-1",
        &[
            sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10),
            sample_local_symbol_hit("BetaSymbol", "src/beta.rs", 20),
        ],
    )
    .await;
    publish_repo_content_chunks(
        &service,
        "alpha/repo",
        &[
            repo_document("src/lib.rs", "fn alpha() {}\nlet beta = 1;\n", "rust", 10),
            repo_document("README.md", "# alpha\n", "markdown", 20),
        ],
        "rev-1",
    )
    .await;
    publish_repo_content_chunks(
        &service,
        "beta/repo",
        &[repo_document("src/lib.rs", "fn gamma() {}\n", "rust", 30)],
        "rev-2",
    )
    .await;
    publish_repo_entities(&service, "alpha/repo", "solve", "Shows solve", "rev-1").await;
    publish_repo_entities(&service, "beta/repo", "gamma", "Shows gamma", "rev-2").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let payload = json!({
        "reference_occurrence": snapshot_query(
            &provider,
            "SELECT name, path, line FROM reference_occurrence ORDER BY name"
        ).await,
        "local_symbol": snapshot_query(
            &provider,
            "SELECT name, path, line_start FROM local_symbol ORDER BY name"
        ).await,
        "repo_content_chunk": snapshot_query(
            &provider,
            "SELECT repo_id, title, doc_type, language_tag, kind_tag, path, line_number, line_text FROM repo_content_chunk WHERE path = 'src/lib.rs' ORDER BY repo_id, line_number"
        ).await,
        "repo_entity": snapshot_query(
            &provider,
            "SELECT repo_id, entity_kind, name, path FROM repo_entity WHERE entity_kind = 'symbol' ORDER BY repo_id, name"
        ).await,
    });

    assert_query_json_snapshot("sql_query_surface_payload", payload);
}

#[tokio::test]
async fn sql_flight_provider_snapshots_discovery_surface() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_reference_hits(
        &service,
        "fp-sql-snapshot-catalog-1",
        &[sample_hit("AlphaService", "src/lib.rs", 10)],
    )
    .await;
    publish_local_symbol_hits(
        &service,
        "fp-sql-snapshot-catalog-2",
        &[sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10)],
    )
    .await;
    publish_repo_content_chunks(
        &service,
        "alpha/repo",
        &[repo_document("src/lib.rs", "fn alpha() {}\n", "rust", 10)],
        "rev-1",
    )
    .await;
    publish_repo_entities(&service, "alpha/repo", "solve", "Shows solve", "rev-1").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let payload = json!({
        "tables_catalog": snapshot_query(
            &provider,
            format!(
                "SELECT sql_table_name, corpus, scope, sql_object_kind, source_count, repo_id FROM {STUDIO_SQL_CATALOG_TABLE_NAME} ORDER BY sql_table_name, COALESCE(repo_id, '')"
            ).as_str()
        ).await,
        "columns_catalog": snapshot_query(
            &provider,
            format!(
                "SELECT sql_table_name, column_name, source_column_name, data_type, sql_object_kind, column_origin_kind FROM {STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME} WHERE sql_table_name IN ('reference_occurrence', '{local_symbol}', '{repo_content_chunk}', '{repo_entity}') ORDER BY sql_table_name, ordinal_position",
                local_symbol = SearchCorpusKind::LocalSymbol,
                repo_content_chunk = SearchCorpusKind::RepoContentChunk,
                repo_entity = SearchCorpusKind::RepoEntity,
            ).as_str()
        ).await,
        "view_sources_catalog": snapshot_query(
            &provider,
            format!(
                "SELECT sql_view_name, source_sql_table_name, corpus, repo_id, source_ordinal FROM {STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME} ORDER BY sql_view_name, source_ordinal, COALESCE(repo_id, '')"
            ).as_str()
        ).await,
        "information_schema_tables": snapshot_query(
            &provider,
            format!(
                "SELECT table_name, table_type FROM information_schema.tables WHERE table_name IN ('reference_occurrence', '{local_symbol}', '{repo_content_chunk}', '{repo_entity}', '{catalog}', '{columns}', '{view_sources}') ORDER BY table_name",
                local_symbol = SearchCorpusKind::LocalSymbol,
                repo_content_chunk = SearchCorpusKind::RepoContentChunk,
                repo_entity = SearchCorpusKind::RepoEntity,
                catalog = STUDIO_SQL_CATALOG_TABLE_NAME,
                columns = STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
                view_sources = STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
            ).as_str()
        ).await,
    });

    assert_query_json_snapshot("sql_discovery_surface_payload", payload);
}

async fn snapshot_query(provider: &StudioSqlFlightRouteProvider, query: &str) -> serde_json::Value {
    let response = provider
        .sql_query_batches(query)
        .await
        .unwrap_or_else(|error| panic!("snapshot query `{query}` failed: {error}"));

    json!({
        "query": query,
        "response": sql_response_snapshot(&response),
    })
}
