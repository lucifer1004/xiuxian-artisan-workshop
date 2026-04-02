use std::collections::HashMap;
use std::sync::Arc;

use arrow_flight::decode::FlightRecordBatchStream;
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::{
    Action, Criteria, Empty, FlightData, FlightDescriptor, FlightEndpoint, FlightInfo,
    HandshakeRequest, PollInfo, SchemaResult, Ticket,
};
use async_trait::async_trait;
use futures::stream;
use futures::{StreamExt, TryStreamExt};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use xiuxian_vector::LanceRecordBatch;

use crate::transport::query_contract::{
    ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, GRAPH_NEIGHBORS_ROUTE, REPO_SEARCH_ROUTE,
    SEARCH_AST_ROUTE, SEARCH_ATTACHMENTS_ROUTE, SEARCH_AUTOCOMPLETE_ROUTE,
    SEARCH_DEFINITION_ROUTE, VFS_RESOLVE_ROUTE, validate_rerank_request_batch,
    validate_rerank_response_batch,
};

use super::request_metadata::{
    descriptor_route, is_search_family_route, join_sorted_set, ticket_route,
    validate_attachment_search_request_metadata, validate_autocomplete_request_metadata,
    validate_code_ast_analysis_request_metadata, validate_definition_request_metadata,
    validate_graph_neighbors_request_metadata, validate_markdown_analysis_request_metadata,
    validate_repo_search_request_metadata, validate_rerank_dimension_header,
    validate_rerank_min_final_score_header, validate_rerank_top_k_header,
    validate_schema_version, validate_search_request_metadata,
    validate_vfs_resolve_request_metadata,
};
use super::types::{
    ActionResultStream, ActionTypeStream, AstSearchFlightRouteProvider,
    AttachmentSearchFlightRouteProvider, AutocompleteFlightRouteProvider,
    CodeAstAnalysisFlightRouteProvider, DefinitionFlightRouteProvider, FlightDataStream,
    FlightInfoStream, GraphNeighborsFlightRouteProvider, HandshakeStream,
    MarkdownAnalysisFlightRouteProvider, PutResultStream, RepoSearchFlightRouteProvider,
    RerankFlightRouteHandler, SearchFlightRouteProvider, StaticRepoSearchFlightRouteProvider,
    VfsResolveFlightRouteProvider,
};
use crate::transport::RerankScoreWeights;

const MAX_PENDING_ROUTE_PAYLOADS: usize = 128;

/// Runtime-owned minimal Wendao Flight service surface for the stable query and
/// rerank routes.
#[derive(Debug, Clone)]
pub struct WendaoFlightService {
    pub(super) expected_schema_version: String,
    repo_search_provider: Arc<dyn RepoSearchFlightRouteProvider>,
    search_provider: Option<Arc<dyn SearchFlightRouteProvider>>,
    attachment_search_provider: Option<Arc<dyn AttachmentSearchFlightRouteProvider>>,
    ast_search_provider: Option<Arc<dyn AstSearchFlightRouteProvider>>,
    definition_provider: Option<Arc<dyn DefinitionFlightRouteProvider>>,
    autocomplete_provider: Option<Arc<dyn AutocompleteFlightRouteProvider>>,
    vfs_resolve_provider: Option<Arc<dyn VfsResolveFlightRouteProvider>>,
    graph_neighbors_provider: Option<Arc<dyn GraphNeighborsFlightRouteProvider>>,
    markdown_analysis_provider: Option<Arc<dyn MarkdownAnalysisFlightRouteProvider>>,
    code_ast_analysis_provider: Option<Arc<dyn CodeAstAnalysisFlightRouteProvider>>,
    rerank_handler: RerankFlightRouteHandler,
    route_payload_cache: Arc<FlightRoutePayloadCache>,
}

#[derive(Debug, Clone)]
struct FlightRoutePayload {
    batch: LanceRecordBatch,
    app_metadata: Vec<u8>,
}

impl FlightRoutePayload {
    fn new(batch: LanceRecordBatch) -> Self {
        Self {
            batch,
            app_metadata: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
struct FlightRoutePayloadCache {
    payloads: Mutex<HashMap<String, FlightRoutePayload>>,
}

impl FlightRoutePayloadCache {
    async fn insert(&self, cache_key: String, payload: FlightRoutePayload) {
        let mut payloads = self.payloads.lock().await;
        if payloads.len() >= MAX_PENDING_ROUTE_PAYLOADS {
            payloads.clear();
        }
        payloads.insert(cache_key, payload);
    }

    async fn take(&self, cache_key: &str) -> Option<FlightRoutePayload> {
        self.payloads.lock().await.remove(cache_key)
    }
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
        Self::new_with_weights(
            expected_schema_version,
            query_response_batch,
            rerank_dimension,
            RerankScoreWeights::default(),
        )
    }

    /// Create one runtime-owned Wendao Flight service with explicit rerank
    /// score weights.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema version is blank or the rerank route
    /// handler configuration is invalid.
    pub fn new_with_weights(
        expected_schema_version: impl Into<String>,
        query_response_batch: LanceRecordBatch,
        rerank_dimension: usize,
        rerank_weights: RerankScoreWeights,
    ) -> Result<Self, String> {
        Self::new_with_provider(
            expected_schema_version,
            Arc::new(StaticRepoSearchFlightRouteProvider {
                batch: query_response_batch,
            }),
            rerank_dimension,
            rerank_weights,
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
        rerank_weights: RerankScoreWeights,
    ) -> Result<Self, String> {
        Self::new_with_route_providers(
            expected_schema_version,
            repo_search_provider,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            rerank_dimension,
            rerank_weights,
        )
    }

    /// Create one runtime-owned Wendao Flight service from pluggable
    /// repo-search and generic search-family providers.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema version is blank or the rerank route
    /// handler configuration is invalid.
    pub fn new_with_route_providers(
        expected_schema_version: impl Into<String>,
        repo_search_provider: Arc<dyn RepoSearchFlightRouteProvider>,
        search_provider: Option<Arc<dyn SearchFlightRouteProvider>>,
        attachment_search_provider: Option<Arc<dyn AttachmentSearchFlightRouteProvider>>,
        ast_search_provider: Option<Arc<dyn AstSearchFlightRouteProvider>>,
        definition_provider: Option<Arc<dyn DefinitionFlightRouteProvider>>,
        autocomplete_provider: Option<Arc<dyn AutocompleteFlightRouteProvider>>,
        markdown_analysis_provider: Option<Arc<dyn MarkdownAnalysisFlightRouteProvider>>,
        code_ast_analysis_provider: Option<Arc<dyn CodeAstAnalysisFlightRouteProvider>>,
        vfs_resolve_provider: Option<Arc<dyn VfsResolveFlightRouteProvider>>,
        graph_neighbors_provider: Option<Arc<dyn GraphNeighborsFlightRouteProvider>>,
        rerank_dimension: usize,
        rerank_weights: RerankScoreWeights,
    ) -> Result<Self, String> {
        let expected_schema_version = expected_schema_version.into();
        if expected_schema_version.trim().is_empty() {
            return Err("wendao flight service schema version must not be blank".to_string());
        }
        Ok(Self {
            expected_schema_version,
            repo_search_provider,
            search_provider,
            attachment_search_provider,
            ast_search_provider,
            definition_provider,
            autocomplete_provider,
            vfs_resolve_provider,
            graph_neighbors_provider,
            markdown_analysis_provider,
            code_ast_analysis_provider,
            rerank_handler: RerankFlightRouteHandler::new_with_weights(
                rerank_dimension,
                rerank_weights,
            )?,
            route_payload_cache: Arc::new(FlightRoutePayloadCache::default()),
        })
    }

    fn route_request_cache_key(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<String, Status> {
        if route == REPO_SEARCH_ROUTE {
            let (
                query_text,
                limit,
                language_filters,
                path_prefixes,
                title_filters,
                tag_filters,
                filename_filters,
            ) = validate_repo_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{}|{}|{}|{}|{}",
                join_sorted_set(&language_filters),
                join_sorted_set(&path_prefixes),
                join_sorted_set(&title_filters),
                join_sorted_set(&tag_filters),
                join_sorted_set(&filename_filters),
            ))
        } else if route == SEARCH_ATTACHMENTS_ROUTE {
            let (query_text, limit, ext_filters, kind_filters, case_sensitive) =
                validate_attachment_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{}|{}|{case_sensitive}",
                join_sorted_set(&ext_filters),
                join_sorted_set(&kind_filters),
            ))
        } else if route == SEARCH_AST_ROUTE {
            let (query_text, limit, intent, repo_hint) =
                validate_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{intent:?}|{repo_hint:?}"
            ))
        } else if route == SEARCH_DEFINITION_ROUTE {
            let (query_text, source_path, source_line) =
                validate_definition_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{source_path:?}|{source_line:?}"
            ))
        } else if route == SEARCH_AUTOCOMPLETE_ROUTE {
            let (prefix, limit) = validate_autocomplete_request_metadata(metadata)?;
            Ok(format!("{route}|{prefix:?}|{limit}"))
        } else if route == VFS_RESOLVE_ROUTE {
            let path = validate_vfs_resolve_request_metadata(metadata)?;
            Ok(format!("{route}|{path:?}"))
        } else if route == GRAPH_NEIGHBORS_ROUTE {
            let (node_id, direction, hops, limit) =
                validate_graph_neighbors_request_metadata(metadata)?;
            Ok(format!("{route}|{node_id:?}|{direction:?}|{hops}|{limit}"))
        } else if route == ANALYSIS_MARKDOWN_ROUTE {
            let path = validate_markdown_analysis_request_metadata(metadata)?;
            Ok(format!("{route}|{path:?}"))
        } else if route == ANALYSIS_CODE_AST_ROUTE {
            let (path, repo_id, line_hint) = validate_code_ast_analysis_request_metadata(metadata)?;
            Ok(format!("{route}|{path:?}|{repo_id:?}|{line_hint:?}"))
        } else if is_search_family_route(route) {
            let (query_text, limit, intent, repo_hint) =
                validate_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{intent:?}|{repo_hint:?}"
            ))
        } else {
            Err(Status::invalid_argument(format!(
                "unexpected routed Flight request: {route}"
            )))
        }
    }

    async fn read_route_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let repo_search_request = validate_repo_search_request_metadata(metadata).ok();
        let search_request = validate_search_request_metadata(metadata).ok();
        let attachment_search_request = validate_attachment_search_request_metadata(metadata).ok();
        let definition_request = validate_definition_request_metadata(metadata).ok();
        let autocomplete_request = validate_autocomplete_request_metadata(metadata).ok();
        let vfs_resolve_request = validate_vfs_resolve_request_metadata(metadata).ok();
        let graph_neighbors_request = validate_graph_neighbors_request_metadata(metadata).ok();
        let markdown_analysis_request = validate_markdown_analysis_request_metadata(metadata).ok();
        let code_ast_analysis_request = validate_code_ast_analysis_request_metadata(metadata).ok();

        if route == REPO_SEARCH_ROUTE {
            let (
                query_text,
                limit,
                language_filters,
                path_prefixes,
                title_filters,
                tag_filters,
                filename_filters,
            ) = repo_search_request.ok_or_else(|| {
                Status::invalid_argument("missing repo-search request metadata for Flight route")
            })?;
            self.repo_search_provider
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
                .map(FlightRoutePayload::new)
                .map_err(Status::internal)
        } else if route == SEARCH_ATTACHMENTS_ROUTE {
            let (query_text, limit, ext_filters, kind_filters, case_sensitive) =
                attachment_search_request.ok_or_else(|| {
                    Status::invalid_argument(
                        "missing attachment-search request metadata for Flight route",
                    )
                })?;
            let provider = self.attachment_search_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "attachment-search Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .attachment_search_batch(
                    query_text.as_str(),
                    limit,
                    &ext_filters,
                    &kind_filters,
                    case_sensitive,
                )
                .await
                .map(FlightRoutePayload::new)
                .map_err(Status::internal)
        } else if route == SEARCH_AST_ROUTE {
            let (query_text, limit, _intent, _repo_hint) = search_request.ok_or_else(|| {
                Status::invalid_argument("missing AST search request metadata for Flight route")
            })?;
            let provider = self.ast_search_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "AST-search Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .ast_search_batch(query_text.as_str(), limit)
                .await
                .map(FlightRoutePayload::new)
                .map_err(Status::internal)
        } else if route == SEARCH_DEFINITION_ROUTE {
            let (query_text, source_path, source_line) = definition_request.ok_or_else(|| {
                Status::invalid_argument("missing definition request metadata for Flight route")
            })?;
            let provider = self.definition_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "definition Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .definition_batch(query_text.as_str(), source_path.as_deref(), source_line)
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
        } else if route == SEARCH_AUTOCOMPLETE_ROUTE {
            let (prefix, limit) = autocomplete_request.ok_or_else(|| {
                Status::invalid_argument("missing autocomplete request metadata for Flight route")
            })?;
            let provider = self.autocomplete_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "autocomplete Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .autocomplete_batch(prefix.as_str(), limit)
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
        } else if route == VFS_RESOLVE_ROUTE {
            let path = vfs_resolve_request.ok_or_else(|| {
                Status::invalid_argument("missing VFS resolve request metadata for Flight route")
            })?;
            let provider = self.vfs_resolve_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "VFS resolve Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .resolve_vfs_navigation_batch(path.as_str())
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
        } else if route == GRAPH_NEIGHBORS_ROUTE {
            let (node_id, direction, hops, limit) = graph_neighbors_request.ok_or_else(|| {
                Status::invalid_argument(
                    "missing graph-neighbors request metadata for Flight route",
                )
            })?;
            let provider = self.graph_neighbors_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "graph-neighbors Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .graph_neighbors_batch(node_id.as_str(), direction.as_str(), hops, limit)
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
        } else if route == ANALYSIS_MARKDOWN_ROUTE {
            let path = markdown_analysis_request.ok_or_else(|| {
                Status::invalid_argument(
                    "missing markdown analysis request metadata for Flight route",
                )
            })?;
            let provider = self.markdown_analysis_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "markdown analysis Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .markdown_analysis_batch(path.as_str())
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
                .map_err(Status::internal)
        } else if route == ANALYSIS_CODE_AST_ROUTE {
            let (path, repo_id, line_hint) = code_ast_analysis_request.ok_or_else(|| {
                Status::invalid_argument(
                    "missing code-AST analysis request metadata for Flight route",
                )
            })?;
            let provider = self.code_ast_analysis_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "code-AST analysis Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .code_ast_analysis_batch(path.as_str(), repo_id.as_str(), line_hint)
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
                .map_err(Status::internal)
        } else if is_search_family_route(route) {
            let (query_text, limit, intent, repo_hint) = search_request.ok_or_else(|| {
                Status::invalid_argument("missing search request metadata for Flight route")
            })?;
            let provider = self.search_provider.as_ref().ok_or_else(|| {
                Status::unimplemented(format!(
                    "search Flight route `{route}` is not configured for this runtime host"
                ))
            })?;
            provider
                .search_batch(
                    route,
                    query_text.as_str(),
                    limit,
                    intent.as_deref(),
                    repo_hint.as_deref(),
                )
                .await
                .map(|response| FlightRoutePayload {
                    batch: response.batch,
                    app_metadata: response.app_metadata,
                })
                .map_err(Status::internal)
        } else {
            Err(Status::invalid_argument(format!(
                "unexpected routed Flight request: {route}"
            )))
        }
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
        let metadata = request.metadata().clone();
        let descriptor = request.into_inner();
        let route = descriptor_route(&descriptor)?;
        let cache_key = self.route_request_cache_key(route.as_str(), &metadata)?;
        let route_payload = self.read_route_payload(route.as_str(), &metadata).await?;
        self.route_payload_cache
            .insert(cache_key, route_payload.clone())
            .await;
        let endpoint = FlightEndpoint::new().with_ticket(Ticket::new(route.clone()));
        let flight_info = FlightInfo::new()
            .try_with_schema(route_payload.batch.schema().as_ref())
            .map_err(|error| Status::internal(error.to_string()))?
            .with_endpoint(endpoint)
            .with_descriptor(descriptor)
            .with_total_records(i64::from(route_payload.batch.num_rows() as i32))
            .with_app_metadata(route_payload.app_metadata);
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
        let metadata = request.metadata().clone();
        let ticket = request.into_inner();
        let route = ticket_route(&ticket)?;
        let cache_key = self.route_request_cache_key(route.as_str(), &metadata)?;
        let query_response_batch =
            if let Some(cached) = self.route_payload_cache.take(&cache_key).await {
                cached.batch
            } else {
                self.read_route_payload(route.as_str(), &metadata)
                    .await?
                    .batch
            };

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
        let top_k = validate_rerank_top_k_header(request.metadata())?;
        let min_final_score = validate_rerank_min_final_score_header(request.metadata())?;

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
            .handle_exchange_batches(&request_batches, top_k, min_final_score)
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
