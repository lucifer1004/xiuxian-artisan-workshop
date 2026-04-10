use arrow_flight::FlightDescriptor;
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::sql::{CommandStatementQuery, ProstMessageExt};
use prost::Message;
use tempfile::TempDir;
use tonic::Request;

#[cfg(feature = "duckdb")]
use crate::duckdb::LocalRelationEngineKind;
#[cfg(feature = "duckdb")]
use crate::search::queries::SearchQueryService;
use crate::search::queries::flightsql::build_studio_flightsql_service;
#[cfg(feature = "duckdb")]
use crate::search::queries::flightsql::execution::{
    FlightSqlStatementRoute, execute_flightsql_statement_query,
};
#[cfg(feature = "duckdb")]
use crate::search::queries::sql::registration::STUDIO_SQL_CATALOG_TABLE_NAME;
#[cfg(feature = "duckdb")]
use crate::search::{SearchCorpusKind, SearchPlaneService};

#[cfg(feature = "duckdb")]
use super::fixtures::write_search_duckdb_runtime_override;
use super::fixtures::{
    collect_flight_frames, decode_flight_batches, fixture_service, publish_reference_hits,
    sample_hit, string_value,
};
#[cfg(feature = "duckdb")]
use super::fixtures::{publish_local_symbol_hits, sample_local_symbol_hit};
#[cfg(feature = "duckdb")]
use super::fixtures::{publish_repo_content_chunks, repo_document};

#[tokio::test]
async fn flightsql_statement_query_returns_reference_occurrence_rows() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "build-1",
        &[sample_hit("AlphaService", "src/alpha.rs", 11)],
    )
    .await;
    let service = build_studio_flightsql_service(search_plane);
    let descriptor = FlightDescriptor::new_cmd(
        CommandStatementQuery {
            query: "SELECT name, path FROM reference_occurrence ORDER BY name".to_string(),
            transaction_id: None,
        }
        .as_any()
        .encode_to_vec(),
    );

    let flight_info = FlightService::get_flight_info(&service, Request::new(descriptor))
        .await
        .unwrap_or_else(|error| panic!("get statement flight info: {error}"))
        .into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.clone())
        .unwrap_or_else(|| panic!("statement flight info should expose a ticket"));
    let frames = collect_flight_frames(
        FlightService::do_get(&service, Request::new(ticket))
            .await
            .unwrap_or_else(|error| panic!("do_get statement: {error}"))
            .into_inner(),
    )
    .await;
    let batches = decode_flight_batches(frames).await;

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 1);
    assert_eq!(string_value(&batches[0], "name", 0), "AlphaService");
    assert_eq!(string_value(&batches[0], "path", 0), "src/alpha.rs");
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn flightsql_statement_query_routes_reference_occurrence_through_local_parquet_duckdb() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "build-1",
        &[sample_hit("AlphaService", "src/alpha.rs", 11)],
    )
    .await;
    let query_service = SearchQueryService::from(search_plane);

    let result = execute_flightsql_statement_query(
        &query_service,
        None,
        "SELECT name, path FROM reference_occurrence ORDER BY name",
    )
    .await
    .unwrap_or_else(|error| panic!("execute FlightSQL statement query: {error}"));

    assert!(matches!(
        result.route,
        FlightSqlStatementRoute::LocalParquet {
            corpus: SearchCorpusKind::ReferenceOccurrence,
            ref table_name,
            engine_kind: LocalRelationEngineKind::DuckDb,
        } if table_name == "reference_occurrence"
    ));
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.batches[0].num_rows(), 1);
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn flightsql_statement_query_routes_catalog_queries_through_shared_sql_duckdb() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "build-1",
        &[sample_hit("AlphaService", "src/alpha.rs", 11)],
    )
    .await;
    let query_service = SearchQueryService::from(search_plane);

    let result = execute_flightsql_statement_query(
        &query_service,
        None,
        format!(
            "SELECT sql_table_name FROM {STUDIO_SQL_CATALOG_TABLE_NAME} ORDER BY sql_table_name LIMIT 1"
        )
        .as_str(),
    )
    .await
    .unwrap_or_else(|error| panic!("execute FlightSQL catalog statement query: {error}"));

    assert_eq!(
        result.route,
        FlightSqlStatementRoute::SharedSql {
            engine_kind: LocalRelationEngineKind::DuckDb,
        }
    );
    assert_eq!(result.batches.len(), 1);
    assert!(result.batches[0].num_rows() >= 1);
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn flightsql_statement_query_routes_repo_content_chunk_source_table_through_published_parquet_duckdb()
 {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let search_plane = fixture_service(&temp_dir);
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
    let table_name = SearchPlaneService::repo_content_chunk_table_name("alpha/repo");
    let query_service = SearchQueryService::from(search_plane);

    let result = execute_flightsql_statement_query(
        &query_service,
        None,
        format!("SELECT path, line_number FROM {table_name} ORDER BY path, line_number").as_str(),
    )
    .await
    .unwrap_or_else(|error| panic!("execute repo source-table FlightSQL statement query: {error}"));

    assert!(matches!(
        result.route,
        FlightSqlStatementRoute::LocalParquet {
            corpus: SearchCorpusKind::RepoContentChunk,
            ref table_name,
            engine_kind: LocalRelationEngineKind::DuckDb,
        } if table_name == SearchPlaneService::repo_content_chunk_table_name("alpha/repo").as_str()
    ));
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.batches[0].num_rows(), 1);
    assert_eq!(string_value(&result.batches[0], "path", 0), "src/lib.rs");
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn flightsql_statement_query_routes_local_symbol_source_table_through_published_parquet_duckdb()
 {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_local_symbol_hits(
        &search_plane,
        "fp-local-symbol-flight-sql",
        &[sample_local_symbol_hit("AlphaSymbol", "src/lib.rs", 10)],
    )
    .await;
    let active_epoch = search_plane
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol)
        .active_epoch
        .unwrap_or_else(|| panic!("active local symbol epoch"));
    let table_name = search_plane
        .local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("local symbol source table"));
    let query_service = SearchQueryService::from(search_plane);

    let result = execute_flightsql_statement_query(
        &query_service,
        None,
        format!("SELECT name, path FROM {table_name} ORDER BY name").as_str(),
    )
    .await
    .unwrap_or_else(|error| {
        panic!("execute local symbol source-table FlightSQL statement query: {error}")
    });

    assert!(matches!(
        result.route,
        FlightSqlStatementRoute::LocalParquet {
            corpus: SearchCorpusKind::LocalSymbol,
            ref table_name,
            engine_kind: LocalRelationEngineKind::DuckDb,
        } if table_name == "local_symbol_epoch_1"
            || table_name.starts_with("local_symbol_epoch_1_part_")
    ));
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.batches[0].num_rows(), 1);
    assert_eq!(string_value(&result.batches[0], "name", 0), "AlphaSymbol");
    assert_eq!(string_value(&result.batches[0], "path", 0), "src/lib.rs");
}

#[tokio::test]
async fn flightsql_statement_query_rejects_transactions_in_first_slice() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = build_studio_flightsql_service(fixture_service(&temp_dir));
    let descriptor = FlightDescriptor::new_cmd(
        CommandStatementQuery {
            query: "SELECT 1".to_string(),
            transaction_id: Some(b"tx-1".to_vec().into()),
        }
        .as_any()
        .encode_to_vec(),
    );

    let Err(error) = FlightService::get_flight_info(&service, Request::new(descriptor)).await
    else {
        panic!("transactions should be rejected in the first slice");
    };
    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert!(error.message().contains("transactions are not implemented"));
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn flightsql_statement_query_reuses_cached_parquet_engine_across_repeated_reference_queries()
{
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let _temp = write_search_duckdb_runtime_override(
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
"#,
    )
    .unwrap_or_else(|error| panic!("write search duckdb runtime override: {error}"));
    let search_plane = fixture_service(&temp_dir);
    publish_reference_hits(
        &search_plane,
        "build-1",
        &[sample_hit("AlphaService", "src/alpha.rs", 11)],
    )
    .await;
    let service = build_studio_flightsql_service(search_plane);

    for _ in 0..2 {
        let descriptor = FlightDescriptor::new_cmd(
            CommandStatementQuery {
                query: "SELECT name, path FROM reference_occurrence ORDER BY name".to_string(),
                transaction_id: None,
            }
            .as_any()
            .encode_to_vec(),
        );

        let flight_info = FlightService::get_flight_info(&service, Request::new(descriptor))
            .await
            .unwrap_or_else(|error| panic!("get repeated statement flight info: {error}"))
            .into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.clone())
            .unwrap_or_else(|| panic!("repeated statement flight info should expose a ticket"));
        let frames = collect_flight_frames(
            FlightService::do_get(&service, Request::new(ticket))
                .await
                .unwrap_or_else(|error| panic!("repeated do_get statement: {error}"))
                .into_inner(),
        )
        .await;
        let batches = decode_flight_batches(frames).await;

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 1);
        assert_eq!(string_value(&batches[0], "name", 0), "AlphaService");
        assert_eq!(string_value(&batches[0], "path", 0), "src/alpha.rs");
    }
}
