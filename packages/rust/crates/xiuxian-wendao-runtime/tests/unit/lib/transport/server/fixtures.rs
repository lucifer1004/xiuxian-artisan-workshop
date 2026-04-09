use std::sync::Arc;

use arrow_flight::FlightData;
use arrow_flight::decode::FlightRecordBatchStream;
use futures::TryStreamExt;
use tonic::Status;

use crate::transport::{RerankScoreWeights, WendaoFlightRouteProviders, WendaoFlightService};

use super::providers::RecordingRepoSearchProvider;

pub(super) fn build_service_with_route_providers(
    configure: impl FnOnce(&mut WendaoFlightRouteProviders),
) -> WendaoFlightService {
    let mut route_providers =
        WendaoFlightRouteProviders::new(Arc::new(RecordingRepoSearchProvider));
    configure(&mut route_providers);
    match WendaoFlightService::new_with_route_providers(
        "v2",
        route_providers,
        3,
        RerankScoreWeights::default(),
    ) {
        Ok(service) => service,
        Err(error) => {
            panic!("service should build from the configured route providers: {error}")
        }
    }
}

pub(super) async fn decode_flight_batches(
    frames: Vec<Result<FlightData, Status>>,
) -> Vec<arrow_array::RecordBatch> {
    let stream = futures::stream::iter(
        frames
            .into_iter()
            .map(|frame| frame.map_err(arrow_flight::error::FlightError::from)),
    );
    let mut batch_stream = FlightRecordBatchStream::new_from_flight_data(stream);
    let mut batches = Vec::new();
    while let Some(batch) = batch_stream
        .try_next()
        .await
        .unwrap_or_else(|error| panic!("decode Flight batches: {error}"))
    {
        batches.push(batch);
    }
    batches
}
