use arrow_flight::FlightDescriptor;
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::sql::{CommandGetSqlInfo, ProstMessageExt, SqlInfo};
use prost::Message;
use tempfile::TempDir;
use tonic::Request;

use crate::search::queries::flightsql::build_studio_flightsql_service;

use super::fixtures::{
    collect_flight_frames, decode_flight_batches, fixture_service, string_value,
};

#[tokio::test]
async fn flightsql_sql_info_reports_wendao_server_identity() {
    let temp_dir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = build_studio_flightsql_service(fixture_service(&temp_dir));
    let descriptor = FlightDescriptor::new_cmd(
        CommandGetSqlInfo {
            info: vec![
                SqlInfo::FlightSqlServerName as u32,
                SqlInfo::FlightSqlServerVersion as u32,
            ],
        }
        .as_any()
        .encode_to_vec(),
    );

    let flight_info = FlightService::get_flight_info(&service, Request::new(descriptor))
        .await
        .unwrap_or_else(|error| panic!("get sql_info flight info: {error}"))
        .into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.clone())
        .unwrap_or_else(|| panic!("sql_info flight info should expose a ticket"));
    let frames = collect_flight_frames(
        FlightService::do_get(&service, Request::new(ticket))
            .await
            .unwrap_or_else(|error| panic!("do_get sql_info: {error}"))
            .into_inner(),
    )
    .await;
    let batches = decode_flight_batches(frames).await;
    let values = (0..batches[0].num_rows())
        .map(|row_index| string_value(&batches[0], "value", row_index))
        .collect::<Vec<_>>();

    assert!(
        values
            .iter()
            .any(|value| value.contains("Wendao FlightSQL Server"))
    );
    assert!(
        values
            .iter()
            .any(|value| value.contains(env!("CARGO_PKG_VERSION")))
    );
}
