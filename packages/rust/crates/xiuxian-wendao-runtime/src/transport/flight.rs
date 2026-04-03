use std::sync::Arc;
use std::time::Duration;

use arrow_flight::FlightDescriptor;
use arrow_flight::client::FlightClient;
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_schema::DataType;
use futures::{TryStreamExt, stream};
use tokio::sync::Mutex;
use tonic::transport::Endpoint;
use xiuxian_vector::EngineRecordBatch;

use super::query_contract::{
    RERANK_REQUEST_EMBEDDING_COLUMN, RERANK_ROUTE, WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_SCHEMA_VERSION_HEADER, flight_descriptor_path, normalize_flight_route,
};

/// Lazy Arrow Flight client aligned to the workspace Arrow Flight transport line.
#[derive(Clone)]
pub(crate) struct ArrowFlightTransportClient {
    base_url: String,
    route: String,
    schema_version: String,
    #[cfg(test)]
    timeout: Duration,
    endpoint: Endpoint,
    client: Arc<Mutex<Option<FlightClient>>>,
}

impl ArrowFlightTransportClient {
    /// Create one lazy Arrow Flight client.
    ///
    /// # Errors
    ///
    /// Returns an error when the base URL, route, schema version, or timeout
    /// cannot be represented as a valid Flight transport configuration.
    pub(crate) fn new(
        base_url: impl Into<String>,
        route: impl Into<String>,
        schema_version: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        if timeout.is_zero() {
            return Err("Arrow Flight timeout must be greater than zero".to_string());
        }

        let base_url = base_url.into();
        let route = normalize_flight_route(route.into())?;
        let schema_version = schema_version.into();
        if schema_version.trim().is_empty() {
            return Err("Arrow Flight schema version must not be blank".to_string());
        }

        let endpoint = Endpoint::from_shared(base_url.clone())
            .map_err(|error| format!("invalid Arrow Flight base URL `{base_url}`: {error}"))?
            .connect_timeout(timeout)
            .timeout(timeout);

        Ok(Self {
            base_url,
            route,
            schema_version,
            #[cfg(test)]
            timeout,
            endpoint,
            client: Arc::new(Mutex::new(None)),
        })
    }

    /// Return the configured Flight endpoint base URL.
    #[must_use]
    pub(crate) fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    /// Return the configured Flight descriptor route.
    #[must_use]
    pub(crate) fn route(&self) -> &str {
        self.route.as_str()
    }

    /// Return the configured schema version metadata value.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn schema_version(&self) -> &str {
        self.schema_version.as_str()
    }

    /// Return the configured request timeout.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Send one Arrow engine batch through the Flight transport.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be converted onto the
    /// workspace Arrow Flight transport line or when the Flight request fails.
    pub(crate) async fn process_batch(
        &self,
        batch: &EngineRecordBatch,
    ) -> Result<Vec<EngineRecordBatch>, String> {
        self.process_batches(std::slice::from_ref(batch)).await
    }

    /// Send multiple Arrow engine batches through the Flight transport.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be converted onto the
    /// workspace Arrow Flight transport line or when the Flight request fails.
    pub(crate) async fn process_batches(
        &self,
        batches: &[EngineRecordBatch],
    ) -> Result<Vec<EngineRecordBatch>, String> {
        if batches.is_empty() {
            return Err("Arrow Flight request batches cannot be empty".to_string());
        }

        let rerank_dimension_header = rerank_dimension_header(self.route.as_str(), batches)?;
        let request_batches = batches.to_vec();
        let request_stream = FlightDataEncoderBuilder::new()
            .with_flight_descriptor(Some(flight_descriptor(self.route.as_str())))
            .build(stream::iter(request_batches.into_iter().map(
                Ok::<EngineRecordBatch, arrow_flight::error::FlightError>,
            )));

        let response = {
            let mut client = self.client.lock().await;
            if client.is_none() {
                let channel =
                    self.endpoint.clone().connect().await.map_err(|error| {
                        format!("failed to connect Arrow Flight endpoint: {error}")
                    })?;
                let mut flight_client = FlightClient::new(channel);
                flight_client
                    .add_header(WENDAO_SCHEMA_VERSION_HEADER, self.schema_version.as_str())
                    .map_err(|error| {
                        format!("invalid Arrow Flight schema-version metadata: {error}")
                    })?;
                if let Some(rerank_dimension_header) = rerank_dimension_header.as_deref() {
                    flight_client
                        .add_header(WENDAO_RERANK_DIMENSION_HEADER, rerank_dimension_header)
                        .map_err(|error| {
                            format!("invalid Arrow Flight rerank-dimension metadata: {error}")
                        })?;
                }
                *client = Some(flight_client);
            }

            let Some(flight_client) = client.as_mut() else {
                return Err(
                    "Arrow Flight client initialization unexpectedly returned no client"
                        .to_string(),
                );
            };
            flight_client
                .do_exchange(request_stream)
                .await
                .map_err(|error| format!("Arrow Flight request failed: {error}"))?
        };

        let response_batches = response
            .try_collect::<Vec<EngineRecordBatch>>()
            .await
            .map_err(|error| format!("failed to decode Arrow Flight response: {error}"))?;
        Ok(response_batches)
    }
}

fn flight_descriptor(route: &str) -> FlightDescriptor {
    let path = flight_descriptor_path(route).unwrap_or_else(|error| {
        panic!("flight descriptor route should already be normalized: {error}")
    });
    FlightDescriptor::new_path(path)
}

fn rerank_dimension_header(
    route: &str,
    request_batches: &[EngineRecordBatch],
) -> Result<Option<String>, String> {
    if route != RERANK_ROUTE {
        return Ok(None);
    }

    let first_batch = request_batches
        .first()
        .ok_or_else(|| "Arrow Flight request batches cannot be empty".to_string())?;
    let embedding_column = first_batch
        .column_by_name(RERANK_REQUEST_EMBEDDING_COLUMN)
        .ok_or_else(|| {
            format!("rerank Flight request missing `{RERANK_REQUEST_EMBEDDING_COLUMN}` column")
        })?;
    match embedding_column.data_type() {
        DataType::FixedSizeList(_, dimension) if *dimension > 0 => Ok(Some(dimension.to_string())),
        other => Err(format!(
            "rerank Flight request column `{RERANK_REQUEST_EMBEDDING_COLUMN}` must be FixedSizeList, found {other:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ArrowFlightTransportClient;
    use crate::transport::{
        REPO_SEARCH_DOC_ID_COLUMN, REPO_SEARCH_LANGUAGE_COLUMN, REPO_SEARCH_PATH_COLUMN,
        REPO_SEARCH_SCORE_COLUMN, REPO_SEARCH_TITLE_COLUMN, RERANK_ROUTE, WendaoFlightService,
    };
    use std::sync::Arc;
    use std::time::Duration;

    use arrow_flight::flight_service_server::FlightServiceServer;
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::Server;
    use xiuxian_vector::{
        EngineRecordBatch, LanceDataType, LanceField, LanceFloat64Array,
        LanceFloat64Array as Float64Array, LanceInt32Array as Int32Array, LanceRecordBatch,
        LanceSchema, LanceStringArray as StringArray, engine_batches_to_lance_batches,
    };

    fn build_rerank_request_batch() -> EngineRecordBatch {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};

        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                Field::new("doc_id", DataType::Utf8, false),
                Field::new("vector_score", DataType::Float32, false),
                Field::new(
                    "embedding",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        3,
                    ),
                    false,
                ),
                Field::new(
                    "query_embedding",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        3,
                    ),
                    false,
                ),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["doc-0", "doc-1"])),
                Arc::new(Float32Array::from(vec![0.5_f32, 0.8_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                            Some(vec![Some(0.0_f32), Some(1.0_f32), Some(0.0_f32)]),
                        ],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                            Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                        ],
                        3,
                    ),
                ),
            ],
        )
        .unwrap_or_else(|error| panic!("request batch should build: {error}"))
    }

    #[tokio::test]
    async fn flight_transport_client_roundtrips_batches_over_lance_arrow_line() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener should expose a local address: {error}"));
        let query_response_batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new(REPO_SEARCH_DOC_ID_COLUMN, LanceDataType::Utf8, false),
                LanceField::new(REPO_SEARCH_PATH_COLUMN, LanceDataType::Utf8, false),
                LanceField::new(REPO_SEARCH_TITLE_COLUMN, LanceDataType::Utf8, false),
                LanceField::new(REPO_SEARCH_SCORE_COLUMN, LanceDataType::Float64, false),
                LanceField::new(REPO_SEARCH_LANGUAGE_COLUMN, LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(StringArray::from(vec!["src/lib.rs"])),
                Arc::new(StringArray::from(vec!["Repo Search Result"])),
                Arc::new(LanceFloat64Array::from(vec![0.91_f64])),
                Arc::new(StringArray::from(vec!["rust"])),
            ],
        )
        .unwrap_or_else(|error| panic!("query response batch should build: {error}"));
        let service = WendaoFlightService::new("v2", query_response_batch, 3)
            .unwrap_or_else(|error| panic!("runtime-owned Flight service should build: {error}"));
        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(FlightServiceServer::new(service))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap_or_else(|error| panic!("mock Flight server should serve: {error}"));
        });

        let client = ArrowFlightTransportClient::new(
            format!("http://{address}"),
            RERANK_ROUTE,
            "v2",
            Duration::from_secs(5),
        )
        .unwrap_or_else(|error| panic!("flight client should build: {error}"));
        let request_batch = build_rerank_request_batch();
        let response_batches = client
            .process_batch(&request_batch)
            .await
            .unwrap_or_else(|error| panic!("flight roundtrip should succeed: {error}"));
        let lance_response_batches = engine_batches_to_lance_batches(&response_batches)
            .unwrap_or_else(|error| {
                panic!("response batches should convert onto Lance Arrow: {error}")
            });

        assert_eq!(response_batches.len(), 1);
        assert_eq!(response_batches[0].num_rows(), 2);
        let doc_ids = lance_response_batches[0]
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .unwrap_or_else(|| panic!("response doc_id column should decode as Utf8"));
        let vector_scores = lance_response_batches[0]
            .column_by_name("vector_score")
            .and_then(|column| column.as_any().downcast_ref::<Float64Array>())
            .unwrap_or_else(|| panic!("response vector_score column should decode as Float64"));
        let semantic_scores = lance_response_batches[0]
            .column_by_name("semantic_score")
            .and_then(|column| column.as_any().downcast_ref::<Float64Array>())
            .unwrap_or_else(|| panic!("response semantic_score column should decode as Float64"));
        let final_scores = lance_response_batches[0]
            .column_by_name("final_score")
            .and_then(|column| column.as_any().downcast_ref::<Float64Array>())
            .unwrap_or_else(|| panic!("response final_score column should decode as Float64"));
        let ranks = lance_response_batches[0]
            .column_by_name("rank")
            .and_then(|column| column.as_any().downcast_ref::<Int32Array>())
            .unwrap_or_else(|| panic!("response rank column should decode as Int32"));
        assert_eq!(doc_ids.value(0), "doc-0");
        assert_eq!(doc_ids.value(1), "doc-1");
        assert!((vector_scores.value(0) - 0.5).abs() < 1e-6);
        assert!((vector_scores.value(1) - 0.8).abs() < 1e-6);
        assert!((semantic_scores.value(0) - 1.0).abs() < 1e-6);
        assert!((semantic_scores.value(1) - 0.5).abs() < 1e-6);
        assert!((final_scores.value(0) - 0.8).abs() < 1e-6);
        assert!((final_scores.value(1) - 0.62).abs() < 1e-6);
        assert_eq!(ranks.value(0), 1);
        assert_eq!(ranks.value(1), 2);
        assert_eq!(client.base_url(), format!("http://{address}"));
        assert_eq!(client.route(), RERANK_ROUTE);
        assert_eq!(client.schema_version(), "v2");
        assert_eq!(client.timeout().as_secs(), 5);

        server.abort();
    }
}
