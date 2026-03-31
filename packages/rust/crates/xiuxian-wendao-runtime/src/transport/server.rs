use std::pin::Pin;
use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_flight::decode::FlightRecordBatchStream;
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::{
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightEndpoint, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt, TryStreamExt};
use tonic::{Request, Response, Status};
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceInt32Array, LanceRecordBatch, LanceSchema,
    LanceStringArray,
};

use super::query_contract::{
    REPO_SEARCH_ROUTE, RERANK_RESPONSE_DOC_ID_COLUMN, RERANK_RESPONSE_FINAL_SCORE_COLUMN,
    RERANK_RESPONSE_RANK_COLUMN, WENDAO_REPO_SEARCH_LIMIT_HEADER, WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER, WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER, WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
    WENDAO_RERANK_DIMENSION_HEADER, WENDAO_SCHEMA_VERSION_HEADER, flight_descriptor_path,
    score_rerank_request_batch, validate_repo_search_request, validate_rerank_request_batch,
    validate_rerank_response_batch,
};

type FlightDataStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;
type HandshakeStream = Pin<Box<dyn Stream<Item = Result<HandshakeResponse, Status>> + Send>>;
type PutResultStream = Pin<Box<dyn Stream<Item = Result<PutResult, Status>> + Send>>;
type ActionResultStream = Pin<Box<dyn Stream<Item = Result<arrow_flight::Result, Status>> + Send>>;
type FlightInfoStream = Pin<Box<dyn Stream<Item = Result<FlightInfo, Status>> + Send>>;
type ActionTypeStream = Pin<Box<dyn Stream<Item = Result<ActionType, Status>> + Send>>;

/// Runtime-owned provider contract for stable repo-search Flight reads.
#[async_trait]
pub trait RepoSearchFlightRouteProvider: std::fmt::Debug + Send + Sync {
    /// Resolve a stable repo-search response batch.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested repo-search payload cannot be
    /// materialized for the current runtime host.
    async fn repo_search_batch(
        &self,
        query_text: &str,
        limit: usize,
        language_filters: &std::collections::HashSet<String>,
        path_prefixes: &std::collections::HashSet<String>,
        title_filters: &std::collections::HashSet<String>,
        tag_filters: &std::collections::HashSet<String>,
        filename_filters: &std::collections::HashSet<String>,
    ) -> Result<LanceRecordBatch, String>;
}

#[derive(Debug, Clone)]
struct StaticRepoSearchFlightRouteProvider {
    batch: LanceRecordBatch,
}

#[async_trait]
impl RepoSearchFlightRouteProvider for StaticRepoSearchFlightRouteProvider {
    async fn repo_search_batch(
        &self,
        _query_text: &str,
        _limit: usize,
        _language_filters: &std::collections::HashSet<String>,
        _path_prefixes: &std::collections::HashSet<String>,
        _title_filters: &std::collections::HashSet<String>,
        _tag_filters: &std::collections::HashSet<String>,
        _filename_filters: &std::collections::HashSet<String>,
    ) -> Result<LanceRecordBatch, String> {
        Ok(self.batch.clone())
    }
}

/// Runtime-owned server-side handler for the stable rerank Flight exchange route.
#[derive(Debug, Clone, Copy)]
pub struct RerankFlightRouteHandler {
    expected_dimension: usize,
}

impl RerankFlightRouteHandler {
    /// Create one rerank Flight route handler.
    ///
    /// # Errors
    ///
    /// Returns an error when the expected embedding dimension is zero.
    pub fn new(expected_dimension: usize) -> Result<Self, String> {
        if expected_dimension == 0 {
            return Err("rerank route expected_dimension must be greater than zero".to_string());
        }
        Ok(Self { expected_dimension })
    }

    /// Build one stable rerank response batch from decoded request batches.
    ///
    /// # Errors
    ///
    /// Returns an error when any request batch fails the shared rerank request
    /// contract, when the combined candidate list is empty, or when the
    /// response batch cannot be represented on the Lance Arrow line.
    pub fn handle_exchange_batches(
        &self,
        request_batches: &[RecordBatch],
    ) -> Result<LanceRecordBatch, String> {
        let mut scored_candidates = Vec::new();
        for batch in request_batches {
            scored_candidates.extend(score_rerank_request_batch(batch, self.expected_dimension)?);
        }

        if scored_candidates.is_empty() {
            return Err("rerank request batches must contain at least one row".to_string());
        }

        scored_candidates.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });

        let doc_ids = scored_candidates
            .iter()
            .map(|(doc_id, _)| doc_id.clone())
            .collect::<Vec<_>>();
        let final_scores = scored_candidates
            .iter()
            .map(|(_, score)| *score)
            .collect::<Vec<_>>();
        let ranks = (1..=i32::try_from(scored_candidates.len())
            .map_err(|error| format!("failed to represent rerank response rank range: {error}"))?)
            .collect::<Vec<_>>();

        LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new(RERANK_RESPONSE_DOC_ID_COLUMN, LanceDataType::Utf8, false),
                LanceField::new(
                    RERANK_RESPONSE_FINAL_SCORE_COLUMN,
                    LanceDataType::Float64,
                    false,
                ),
                LanceField::new(RERANK_RESPONSE_RANK_COLUMN, LanceDataType::Int32, false),
            ])),
            vec![
                Arc::new(LanceStringArray::from(doc_ids)),
                Arc::new(LanceFloat64Array::from(final_scores)),
                Arc::new(LanceInt32Array::from(ranks)),
            ],
        )
        .map_err(|error| format!("failed to build rerank response batch: {error}"))
    }
}

/// Runtime-owned minimal Wendao Flight service surface for the stable query and
/// rerank routes.
#[derive(Debug, Clone)]
pub struct WendaoFlightService {
    expected_schema_version: String,
    repo_search_provider: Arc<dyn RepoSearchFlightRouteProvider>,
    rerank_handler: RerankFlightRouteHandler,
}

impl WendaoFlightService {
    /// Create one runtime-owned Wendao Flight service.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema version is blank or the rerank route
    /// handler configuration is invalid.
    pub fn new(
        expected_schema_version: impl Into<String>,
        query_response_batch: LanceRecordBatch,
        rerank_dimension: usize,
    ) -> Result<Self, String> {
        Self::new_with_provider(
            expected_schema_version,
            Arc::new(StaticRepoSearchFlightRouteProvider {
                batch: query_response_batch,
            }),
            rerank_dimension,
        )
    }

    /// Create one runtime-owned Wendao Flight service from a pluggable
    /// repo-search provider.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema version is blank or the rerank route
    /// handler configuration is invalid.
    pub fn new_with_provider(
        expected_schema_version: impl Into<String>,
        repo_search_provider: Arc<dyn RepoSearchFlightRouteProvider>,
        rerank_dimension: usize,
    ) -> Result<Self, String> {
        let expected_schema_version = expected_schema_version.into();
        if expected_schema_version.trim().is_empty() {
            return Err("wendao flight service schema version must not be blank".to_string());
        }
        Ok(Self {
            expected_schema_version,
            repo_search_provider,
            rerank_handler: RerankFlightRouteHandler::new(rerank_dimension)?,
        })
    }
}

#[async_trait]
impl FlightService for WendaoFlightService {
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
        Err(Status::unimplemented(
            "handshake is not used by this Flight service",
        ))
    }

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        Err(Status::unimplemented(
            "list_flights is not used by this Flight service",
        ))
    }

    async fn get_flight_info(
        &self,
        request: Request<FlightDescriptor>,
    ) -> Result<Response<FlightInfo>, Status> {
        validate_schema_version(request.metadata(), self.expected_schema_version.as_str())?;
        let (
            query_text,
            limit,
            language_filters,
            path_prefixes,
            title_filters,
            tag_filters,
            filename_filters,
        ) = validate_repo_search_request_metadata(request.metadata())?;
        let descriptor = request.into_inner();
        validate_descriptor_route(&descriptor, REPO_SEARCH_ROUTE)?;
        let query_response_batch = self
            .repo_search_provider
            .repo_search_batch(
                query_text.as_str(),
                limit,
                &language_filters,
                &path_prefixes,
                &title_filters,
                &tag_filters,
                &filename_filters,
            )
            .await
            .map_err(Status::internal)?;
        let endpoint = FlightEndpoint::new().with_ticket(Ticket::new(REPO_SEARCH_ROUTE));
        let flight_info = FlightInfo::new()
            .try_with_schema(query_response_batch.schema().as_ref())
            .map_err(|error| Status::internal(error.to_string()))?
            .with_endpoint(endpoint)
            .with_descriptor(descriptor)
            .with_total_records(i64::from(query_response_batch.num_rows() as i32));
        Ok(Response::new(flight_info))
    }

    async fn poll_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<PollInfo>, Status> {
        Err(Status::unimplemented(
            "poll_flight_info is not used by this Flight service",
        ))
    }

    async fn get_schema(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<SchemaResult>, Status> {
        Err(Status::unimplemented(
            "get_schema is not used by this Flight service",
        ))
    }

    async fn do_get(
        &self,
        request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        validate_schema_version(request.metadata(), self.expected_schema_version.as_str())?;
        let (
            query_text,
            limit,
            language_filters,
            path_prefixes,
            title_filters,
            tag_filters,
            filename_filters,
        ) = validate_repo_search_request_metadata(request.metadata())?;
        let ticket = request.into_inner();
        if ticket.ticket.as_ref() != REPO_SEARCH_ROUTE.as_bytes() {
            return Err(Status::invalid_argument(format!(
                "unexpected ticket: {:?}",
                ticket.ticket
            )));
        }
        let query_response_batch = self
            .repo_search_provider
            .repo_search_batch(
                query_text.as_str(),
                limit,
                &language_filters,
                &path_prefixes,
                &title_filters,
                &tag_filters,
                &filename_filters,
            )
            .await
            .map_err(Status::internal)?;

        let response_stream = FlightDataEncoderBuilder::new()
            .build(stream::iter(vec![Ok::<
                LanceRecordBatch,
                arrow_flight::error::FlightError,
            >(query_response_batch)]))
            .map(|item| item.map_err(|error| Status::internal(error.to_string())));
        Ok(Response::new(Box::pin(response_stream)))
    }

    async fn do_put(
        &self,
        _request: Request<tonic::Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        Err(Status::unimplemented(
            "do_put is not used by this Flight service",
        ))
    }

    async fn do_exchange(
        &self,
        request: Request<tonic::Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        validate_schema_version(request.metadata(), self.expected_schema_version.as_str())?;
        let expected_dimension = validate_rerank_dimension_header(request.metadata())?;

        let stream = request
            .into_inner()
            .map(|frame| frame.map_err(arrow_flight::error::FlightError::from))
            .try_filter(|frame| futures::future::ready(!frame.data_header.is_empty()));
        let mut batch_stream = FlightRecordBatchStream::new_from_flight_data(stream);
        let mut request_batches = Vec::new();
        while let Some(batch) = batch_stream.try_next().await.map_err(|error| {
            Status::invalid_argument(format!("failed to decode rerank request batch: {error}"))
        })? {
            validate_rerank_request_batch(&batch, expected_dimension)
                .map_err(Status::invalid_argument)?;
            request_batches.push(batch);
        }
        if request_batches.is_empty() {
            return Err(Status::invalid_argument(
                "expected at least one rerank record batch",
            ));
        }

        let exchange_response_batch = self
            .rerank_handler
            .handle_exchange_batches(&request_batches)
            .map_err(Status::invalid_argument)?;
        validate_rerank_response_batch(&exchange_response_batch).map_err(Status::internal)?;

        let response_stream = FlightDataEncoderBuilder::new()
            .build(stream::iter(vec![Ok::<
                LanceRecordBatch,
                arrow_flight::error::FlightError,
            >(exchange_response_batch)]))
            .map(|item| item.map_err(|error| Status::internal(error.to_string())));
        Ok(Response::new(Box::pin(response_stream)))
    }

    async fn do_action(
        &self,
        _request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        Err(Status::unimplemented(
            "do_action is not used by this Flight service",
        ))
    }

    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        Err(Status::unimplemented(
            "list_actions is not used by this Flight service",
        ))
    }
}

fn validate_schema_version(
    metadata: &tonic::metadata::MetadataMap,
    expected_schema_version: &str,
) -> Result<(), Status> {
    let schema_version = metadata
        .get(WENDAO_SCHEMA_VERSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if schema_version != expected_schema_version {
        return Err(Status::invalid_argument(format!(
            "unexpected schema version header: {schema_version}"
        )));
    }
    Ok(())
}

fn validate_rerank_dimension_header(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<usize, Status> {
    let dimension = metadata
        .get(WENDAO_RERANK_DIMENSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let parsed_dimension = dimension.parse::<usize>().map_err(|_| {
        Status::invalid_argument(format!(
            "invalid rerank dimension header `{WENDAO_RERANK_DIMENSION_HEADER}`: {dimension}"
        ))
    })?;
    if parsed_dimension == 0 {
        return Err(Status::invalid_argument(format!(
            "rerank dimension header `{WENDAO_RERANK_DIMENSION_HEADER}` must be greater than zero"
        )));
    }
    Ok(parsed_dimension)
}

fn validate_repo_search_request_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<
    (
        String,
        usize,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
    ),
    Status,
> {
    let query_text = metadata
        .get(WENDAO_REPO_SEARCH_QUERY_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let limit = metadata
        .get(WENDAO_REPO_SEARCH_LIMIT_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let parsed_limit = limit.parse::<usize>().map_err(|_| {
        Status::invalid_argument(format!(
            "invalid repo search limit header `{WENDAO_REPO_SEARCH_LIMIT_HEADER}`: {limit}"
        ))
    })?;
    let language_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let path_prefix_values = metadata
        .get(WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let title_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let tag_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let filename_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty()
                || metadata.contains_key(WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    validate_repo_search_request(
        query_text.as_str(),
        parsed_limit,
        &language_filter_values,
        &path_prefix_values,
        &title_filter_values,
        &tag_filter_values,
        &filename_filter_values,
    )
        .map_err(Status::invalid_argument)?;
    let language_filters = language_filter_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let path_prefixes = path_prefix_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let title_filters = title_filter_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let tag_filters = tag_filter_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let filename_filters = filename_filter_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    Ok((
        query_text,
        parsed_limit,
        language_filters,
        path_prefixes,
        title_filters,
        tag_filters,
        filename_filters,
    ))
}

fn validate_descriptor_route(
    descriptor: &FlightDescriptor,
    expected_route: &str,
) -> Result<(), Status> {
    let expected_path = flight_descriptor_path(expected_route).map_err(Status::invalid_argument)?;
    let actual_path = descriptor
        .path
        .iter()
        .map(|segment| String::from_utf8_lossy(segment.as_ref()).into_owned())
        .collect::<Vec<_>>();
    if actual_path != expected_path {
        let actual_route = format!("/{}", actual_path.join("/"));
        return Err(Status::invalid_argument(format!(
            "unexpected descriptor route: {actual_route}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RepoSearchFlightRouteProvider, RerankFlightRouteHandler, WendaoFlightService};
    use arrow_array::types::Float32Type;
    use arrow_array::{FixedSizeListArray, Float32Array, StringArray};
    use async_trait::async_trait;
    use std::sync::Arc;
    use xiuxian_vector::{
        LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema,
    };

    #[test]
    fn rerank_route_handler_scores_and_ranks_semantic_candidates() {
        let request_batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("vector_score", LanceDataType::Float32, false),
                LanceField::new(
                    "embedding",
                    LanceDataType::FixedSizeList(
                        Arc::new(LanceField::new("item", LanceDataType::Float32, true)),
                        3,
                    ),
                    false,
                ),
                LanceField::new(
                    "query_embedding",
                    LanceDataType::FixedSizeList(
                        Arc::new(LanceField::new("item", LanceDataType::Float32, true)),
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
        .expect("request batch should build");
        let handler = RerankFlightRouteHandler::new(3).expect("handler should build");

        let response = handler
            .handle_exchange_batches(std::slice::from_ref(&request_batch))
            .expect("rerank route handler should score request batches");

        let doc_ids = response
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("doc_id should decode as Utf8");
        let final_scores = response
            .column_by_name("final_score")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Float64Array>())
            .expect("final_score should decode as Float64");
        let ranks = response
            .column_by_name("rank")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Int32Array>())
            .expect("rank should decode as Int32");

        assert_eq!(doc_ids.value(0), "doc-0");
        assert_eq!(doc_ids.value(1), "doc-1");
        assert!((final_scores.value(0) - 0.8).abs() < 1e-6);
        assert!((final_scores.value(1) - 0.62).abs() < 1e-6);
        assert_eq!(ranks.value(0), 1);
        assert_eq!(ranks.value(1), 2);
    }

    #[test]
    fn rerank_route_handler_rejects_zero_dimension() {
        let error = RerankFlightRouteHandler::new(0)
            .expect_err("zero-dimension handler construction should fail");
        assert_eq!(
            error,
            "rerank route expected_dimension must be greater than zero"
        );
    }

    #[test]
    fn wendao_flight_service_rejects_blank_schema_version() {
        let query_response_batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("path", LanceDataType::Utf8, false),
                LanceField::new("title", LanceDataType::Utf8, false),
                LanceField::new("score", LanceDataType::Float64, false),
                LanceField::new("language", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(StringArray::from(vec!["src/lib.rs"])),
                Arc::new(StringArray::from(vec!["Repo Search Result"])),
                Arc::new(LanceFloat64Array::from(vec![0.91_f64])),
                Arc::new(StringArray::from(vec!["rust"])),
            ],
        )
        .expect("query response batch should build");

        let error = WendaoFlightService::new("   ", query_response_batch, 3)
            .expect_err("blank schema-version service construction should fail");
        assert_eq!(
            error,
            "wendao flight service schema version must not be blank"
        );
    }

    #[derive(Debug)]
    struct RecordingRepoSearchProvider;

    #[async_trait]
    impl RepoSearchFlightRouteProvider for RecordingRepoSearchProvider {
        async fn repo_search_batch(
            &self,
            query_text: &str,
            limit: usize,
            _language_filters: &std::collections::HashSet<String>,
            _path_prefixes: &std::collections::HashSet<String>,
            _title_filters: &std::collections::HashSet<String>,
            _tag_filters: &std::collections::HashSet<String>,
            _filename_filters: &std::collections::HashSet<String>,
        ) -> Result<LanceRecordBatch, String> {
            LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("doc_id", LanceDataType::Utf8, false),
                    LanceField::new("path", LanceDataType::Utf8, false),
                    LanceField::new("title", LanceDataType::Utf8, false),
                    LanceField::new("score", LanceDataType::Float64, false),
                    LanceField::new("language", LanceDataType::Utf8, false),
                ])),
                vec![
                    Arc::new(StringArray::from(vec![format!("doc:{query_text}:{limit}")])),
                    Arc::new(StringArray::from(vec!["src/lib.rs"])),
                    Arc::new(StringArray::from(vec!["Repo Search Result"])),
                    Arc::new(LanceFloat64Array::from(vec![0.91_f64])),
                    Arc::new(StringArray::from(vec!["rust"])),
                ],
            )
            .map_err(|error| error.to_string())
        }
    }

    #[test]
    fn wendao_flight_service_accepts_pluggable_repo_search_provider() {
        let service =
            WendaoFlightService::new_with_provider("v2", Arc::new(RecordingRepoSearchProvider), 3)
                .expect("service should build from a pluggable repo-search provider");

        assert_eq!(service.expected_schema_version, "v2");
    }
}
