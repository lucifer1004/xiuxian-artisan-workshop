use arrow_flight::FlightDescriptor;
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::sql::{CommandStatementQuery, ProstMessageExt};
use prost::Message;
use tempfile::TempDir;
use tonic::Request;

use crate::search::queries::flightsql::build_studio_flightsql_service;

use super::fixtures::{
    collect_flight_frames, decode_flight_batches, fixture_service, publish_reference_hits,
    sample_hit, string_value,
};

#[tokio::test]
async fn flightsql_statement_query_routes_through_shared_reference_occurrence_surface() {
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
