use std::sync::Arc;

use arrow_flight::FlightData;
use arrow_flight::encode::FlightDataEncoderBuilder;
use futures::TryStreamExt;
use futures::stream;
use tokio::sync::Mutex;
use tonic::Status;
use xiuxian_vector_store::{EngineRecordBatch, LanceRecordBatch, lance_batches_to_engine_batches};

#[derive(Debug)]
pub(super) struct FlightRoutePayload {
    pub(super) batches: Vec<EngineRecordBatch>,
    pub(super) app_metadata: Vec<u8>,
    encoded_do_get_frames: Mutex<Option<Arc<Vec<FlightData>>>>,
}

impl FlightRoutePayload {
    pub(super) fn try_new(batch: LanceRecordBatch) -> Result<Self, Status> {
        let batches = [batch];
        Self::from_batches_with_app_metadata(&batches, Vec::new())
    }

    pub(super) fn try_with_app_metadata(
        batch: LanceRecordBatch,
        app_metadata: Vec<u8>,
    ) -> Result<Self, Status> {
        let batches = [batch];
        Self::from_batches_with_app_metadata(&batches, app_metadata)
    }

    pub(super) fn from_batches_with_app_metadata(
        batches: &[LanceRecordBatch],
        app_metadata: Vec<u8>,
    ) -> Result<Self, Status> {
        if batches.is_empty() {
            return Err(Status::internal(
                "Flight route payload must contain at least one record batch",
            ));
        }
        let engine_batches = lance_batches_to_engine_batches(batches)
            .map_err(|error| Status::internal(error.to_string()))?;
        Ok(Self {
            batches: engine_batches,
            app_metadata,
            encoded_do_get_frames: Mutex::new(None),
        })
    }

    pub(super) fn schema(&self) -> Arc<arrow_schema::Schema> {
        self.batches[0].schema()
    }

    pub(super) fn total_rows(&self) -> Result<i64, Status> {
        i64::try_from(
            self.batches
                .iter()
                .map(EngineRecordBatch::num_rows)
                .sum::<usize>(),
        )
        .map_err(|error| Status::internal(format!("failed to represent total records: {error}")))
    }

    pub(super) async fn do_get_frames(&self) -> Result<Arc<Vec<FlightData>>, Status> {
        if let Some(cached) = self.encoded_do_get_frames.lock().await.as_ref().cloned() {
            return Ok(cached);
        }

        let encoded = Arc::new(encode_do_get_frames(self.batches.clone()).await?);
        let mut cached_frames = self.encoded_do_get_frames.lock().await;
        if let Some(cached) = cached_frames.as_ref().cloned() {
            return Ok(cached);
        }
        *cached_frames = Some(Arc::clone(&encoded));
        Ok(encoded)
    }
}

async fn encode_do_get_frames(batches: Vec<EngineRecordBatch>) -> Result<Vec<FlightData>, Status> {
    FlightDataEncoderBuilder::new()
        .build(stream::iter(batches.into_iter().map(
            Ok::<EngineRecordBatch, arrow_flight::error::FlightError>,
        )))
        .map_err(|error| Status::internal(error.to_string()))
        .try_collect::<Vec<_>>()
        .await
}
