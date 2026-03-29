use std::sync::Arc;
use std::time::Duration;

use arrow_flight::client::FlightClient;
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_flight::FlightDescriptor;
use futures::{stream, TryStreamExt};
use tokio::sync::Mutex;
use tonic::transport::Endpoint;
use xiuxian_vector::{
    EngineRecordBatch, LanceRecordBatch, engine_batches_to_lance_batches,
    lance_batches_to_engine_batches,
};

const WENDAO_SCHEMA_VERSION_HEADER: &str = "x-wendao-schema-version";

/// Lazy Arrow Flight client aligned to the Lance/Arrow-57 transport line.
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
        let route = normalize_route(route.into())?;
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
    /// Lance/Arrow-57 transport line, when the Flight request fails, or when
    /// the response cannot be converted back into engine batches.
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
    /// Lance/Arrow-57 transport line, when the Flight request fails, or when
    /// the response cannot be converted back into engine batches.
    pub(crate) async fn process_batches(
        &self,
        batches: &[EngineRecordBatch],
    ) -> Result<Vec<EngineRecordBatch>, String> {
        if batches.is_empty() {
            return Err("Arrow Flight request batches cannot be empty".to_string());
        }

        let request_batches = engine_batches_to_lance_batches(batches).map_err(|error| {
            format!("failed to convert engine batches onto the Lance Arrow line: {error}")
        })?;
        let request_stream = FlightDataEncoderBuilder::new()
            .with_flight_descriptor(Some(flight_descriptor(self.route.as_str())))
            .build(stream::iter(
                request_batches
                    .into_iter()
                    .map(Ok::<LanceRecordBatch, arrow_flight::error::FlightError>),
            ));

        let response = {
            let mut client = self.client.lock().await;
            if client.is_none() {
                let channel = self
                    .endpoint
                    .clone()
                    .connect()
                    .await
                    .map_err(|error| format!("failed to connect Arrow Flight endpoint: {error}"))?;
                let mut flight_client = FlightClient::new(channel);
                flight_client
                    .add_header(WENDAO_SCHEMA_VERSION_HEADER, self.schema_version.as_str())
                    .map_err(|error| {
                        format!("invalid Arrow Flight schema-version metadata: {error}")
                    })?;
                *client = Some(flight_client);
            }

            client
                .as_mut()
                .expect("flight client initialized above")
                .do_exchange(request_stream)
                .await
                .map_err(|error| format!("Arrow Flight request failed: {error}"))?
        };

        let response_batches = response
            .try_collect::<Vec<LanceRecordBatch>>()
            .await
            .map_err(|error| format!("failed to decode Arrow Flight response: {error}"))?;
        lance_batches_to_engine_batches(&response_batches).map_err(|error| {
            format!("failed to convert Arrow Flight response onto the engine Arrow line: {error}")
        })
    }
}

fn normalize_route(route: String) -> Result<String, String> {
    let normalized = if route.starts_with('/') {
        route
    } else {
        format!("/{route}")
    };
    if normalized.trim_matches('/').is_empty() {
        return Err("Arrow Flight route must resolve to at least one descriptor segment".to_string());
    }
    Ok(normalized)
}

fn flight_descriptor(route: &str) -> FlightDescriptor {
    let path = route
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    FlightDescriptor::new_path(path)
}

#[cfg(test)]
mod tests {
    use super::{ArrowFlightTransportClient, WENDAO_SCHEMA_VERSION_HEADER};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;

    use arrow_flight::encode::FlightDataEncoderBuilder;
    use arrow_flight::flight_service_server::{FlightService, FlightServiceServer};
    use arrow_flight::{
        Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
        HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
    };
    use async_trait::async_trait;
    use futures::stream;
    use futures::StreamExt;
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::Server;
    use tonic::{Request, Response, Status};
    use xiuxian_vector::{
        LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray,
        lance_batch_to_engine_batch,
    };

    #[derive(Clone)]
    struct MockFlightService {
        response_batch: LanceRecordBatch,
    }

    type FlightDataStream = Pin<Box<dyn futures::Stream<Item = Result<FlightData, Status>> + Send>>;
    type HandshakeStream = Pin<Box<dyn futures::Stream<Item = Result<HandshakeResponse, Status>> + Send>>;
    type PutResultStream = Pin<Box<dyn futures::Stream<Item = Result<PutResult, Status>> + Send>>;
    type ActionResultStream =
        Pin<Box<dyn futures::Stream<Item = Result<arrow_flight::Result, Status>> + Send>>;
    type FlightInfoStream = Pin<Box<dyn futures::Stream<Item = Result<FlightInfo, Status>> + Send>>;
    type ActionTypeStream =
        Pin<Box<dyn futures::Stream<Item = Result<ActionType, Status>> + Send>>;

    #[async_trait]
    impl FlightService for MockFlightService {
        type HandshakeStream = HandshakeStream;
        type ListFlightsStream = FlightInfoStream;
        type DoGetStream = FlightDataStream;
        type DoPutStream = PutResultStream;
        type DoExchangeStream = FlightDataStream;
        type DoActionStream = ActionResultStream;
        type ListActionsStream = ActionTypeStream;

        async fn handshake(
            &self,
            _request: Request<tonic::Streaming<HandshakeRequest>>,
        ) -> Result<Response<Self::HandshakeStream>, Status> {
            Err(Status::unimplemented("handshake is not used in this test"))
        }

        async fn list_flights(
            &self,
            _request: Request<Criteria>,
        ) -> Result<Response<Self::ListFlightsStream>, Status> {
            Err(Status::unimplemented("list_flights is not used in this test"))
        }

        async fn get_flight_info(
            &self,
            _request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            Err(Status::unimplemented("get_flight_info is not used in this test"))
        }

        async fn poll_flight_info(
            &self,
            _request: Request<FlightDescriptor>,
        ) -> Result<Response<PollInfo>, Status> {
            Err(Status::unimplemented("poll_flight_info is not used in this test"))
        }

        async fn get_schema(
            &self,
            _request: Request<FlightDescriptor>,
        ) -> Result<Response<SchemaResult>, Status> {
            Err(Status::unimplemented("get_schema is not used in this test"))
        }

        async fn do_get(
            &self,
            _request: Request<Ticket>,
        ) -> Result<Response<Self::DoGetStream>, Status> {
            Err(Status::unimplemented("do_get is not used in this test"))
        }

        async fn do_put(
            &self,
            _request: Request<tonic::Streaming<FlightData>>,
        ) -> Result<Response<Self::DoPutStream>, Status> {
            Err(Status::unimplemented("do_put is not used in this test"))
        }

        async fn do_exchange(
            &self,
            request: Request<tonic::Streaming<FlightData>>,
        ) -> Result<Response<Self::DoExchangeStream>, Status> {
            let schema_version = request
                .metadata()
                .get(WENDAO_SCHEMA_VERSION_HEADER)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            if schema_version != "v2" {
                return Err(Status::invalid_argument(format!(
                    "unexpected schema version header: {schema_version}"
                )));
            }

            let mut stream = request.into_inner();
            if stream.message().await?.is_none() {
                return Err(Status::invalid_argument("expected at least one request frame"));
            }

            let response_stream = FlightDataEncoderBuilder::new()
                .build(stream::iter(vec![Ok::<LanceRecordBatch, arrow_flight::error::FlightError>(
                    self.response_batch.clone(),
                )]))
                .map(|item| item.map_err(|error| Status::internal(error.to_string())));
            Ok(Response::new(Box::pin(response_stream)))
        }

        async fn do_action(
            &self,
            _request: Request<Action>,
        ) -> Result<Response<Self::DoActionStream>, Status> {
            Err(Status::unimplemented("do_action is not used in this test"))
        }

        async fn list_actions(
            &self,
            _request: Request<Empty>,
        ) -> Result<Response<Self::ListActionsStream>, Status> {
            Err(Status::unimplemented("list_actions is not used in this test"))
        }
    }

    #[tokio::test]
    async fn flight_transport_client_roundtrips_batches_over_lance_arrow_line() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener should expose a local address: {error}"));
        let response_batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![LanceField::new(
                "doc_id",
                LanceDataType::Utf8,
                false,
            )])),
            vec![Arc::new(LanceStringArray::from(vec!["doc-1"]))],
        )
        .unwrap_or_else(|error| panic!("response batch should build: {error}"));
        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(FlightServiceServer::new(MockFlightService { response_batch }))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap_or_else(|error| panic!("mock Flight server should serve: {error}"));
        });

        let client = ArrowFlightTransportClient::new(
            format!("http://{address}"),
            "/rerank/flight",
            "v2",
            Duration::from_secs(5),
        )
        .unwrap_or_else(|error| panic!("flight client should build: {error}"));
        let request_batch = lance_batch_to_engine_batch(&LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![LanceField::new(
                "doc_id",
                LanceDataType::Utf8,
                false,
            )])),
            vec![Arc::new(LanceStringArray::from(vec!["doc-0"]))],
        )
        .unwrap_or_else(|error| panic!("request batch should build: {error}")))
        .unwrap_or_else(|error| panic!("request batch should convert onto engine Arrow: {error}"));
        let response_batches = client
            .process_batch(&request_batch)
            .await
            .unwrap_or_else(|error| panic!("flight roundtrip should succeed: {error}"));

        assert_eq!(response_batches.len(), 1);
        assert_eq!(response_batches[0].num_rows(), 1);
        assert_eq!(client.base_url(), format!("http://{address}"));
        assert_eq!(client.route(), "/rerank/flight");
        assert_eq!(client.schema_version(), "v2");
        assert_eq!(client.timeout().as_secs(), 5);

        server.abort();
    }
}
