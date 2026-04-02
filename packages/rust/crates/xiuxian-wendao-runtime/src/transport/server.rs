use std::collections::HashMap;
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
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceInt32Array, LanceRecordBatch, LanceSchema,
    LanceStringArray,
};

use super::query_contract::{
    ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, REPO_SEARCH_ROUTE,
    RERANK_RESPONSE_DOC_ID_COLUMN, RERANK_RESPONSE_FINAL_SCORE_COLUMN, RERANK_RESPONSE_RANK_COLUMN,
    RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN, RERANK_RESPONSE_VECTOR_SCORE_COLUMN, RerankScoreWeights,
    SEARCH_AST_ROUTE, SEARCH_ATTACHMENTS_ROUTE, SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE,
    SEARCH_REFERENCES_ROUTE, SEARCH_SYMBOLS_ROUTE, WENDAO_ANALYSIS_LINE_HEADER,
    WENDAO_ANALYSIS_PATH_HEADER, WENDAO_ANALYSIS_REPO_HEADER,
    WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER, WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER,
    WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER, WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER, WENDAO_REPO_SEARCH_LIMIT_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER, WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER, WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
    WENDAO_RERANK_DIMENSION_HEADER, WENDAO_RERANK_MIN_FINAL_SCORE_HEADER,
    WENDAO_RERANK_TOP_K_HEADER, WENDAO_SCHEMA_VERSION_HEADER, WENDAO_SEARCH_INTENT_HEADER,
    WENDAO_SEARCH_LIMIT_HEADER, WENDAO_SEARCH_QUERY_HEADER, WENDAO_SEARCH_REPO_HEADER,
    normalize_flight_route, score_rerank_request_batch_with_weights,
    validate_attachment_search_request, validate_code_ast_analysis_request,
    validate_markdown_analysis_request, validate_repo_search_request,
    validate_rerank_request_batch, validate_rerank_response_batch,
};

type FlightDataStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;
type HandshakeStream = Pin<Box<dyn Stream<Item = Result<HandshakeResponse, Status>> + Send>>;
type PutResultStream = Pin<Box<dyn Stream<Item = Result<PutResult, Status>> + Send>>;
type ActionResultStream = Pin<Box<dyn Stream<Item = Result<arrow_flight::Result, Status>> + Send>>;
type FlightInfoStream = Pin<Box<dyn Stream<Item = Result<FlightInfo, Status>> + Send>>;
type ActionTypeStream = Pin<Box<dyn Stream<Item = Result<ActionType, Status>> + Send>>;
const MAX_PENDING_ROUTE_PAYLOADS: usize = 128;

/// Runtime-owned generic search-family Flight payload.
#[derive(Debug, Clone)]
pub struct SearchFlightRouteResponse {
    /// Arrow batch returned by the provider.
    pub batch: LanceRecordBatch,
    /// Optional application metadata returned through `FlightInfo.app_metadata`.
    pub app_metadata: Vec<u8>,
}

impl SearchFlightRouteResponse {
    /// Create one search-family Flight payload without application metadata.
    #[must_use]
    pub fn new(batch: LanceRecordBatch) -> Self {
        Self {
            batch,
            app_metadata: Vec::new(),
        }
    }

    /// Attach application metadata that should flow through `FlightInfo`.
    #[must_use]
    pub fn with_app_metadata(mut self, app_metadata: impl Into<Vec<u8>>) -> Self {
        self.app_metadata = app_metadata.into();
        self
    }
}

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

/// Runtime-owned provider contract for stable generic search-family Flight
/// reads.
#[async_trait]
pub trait SearchFlightRouteProvider: std::fmt::Debug + Send + Sync {
    /// Resolve one stable search-family response batch for the requested route.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested search-family payload cannot be
    /// materialized for the current runtime host.
    async fn search_batch(
        &self,
        route: &str,
        query_text: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Result<SearchFlightRouteResponse, String>;
}

/// Runtime-owned provider contract for stable attachment-search Flight reads.
#[async_trait]
pub trait AttachmentSearchFlightRouteProvider: std::fmt::Debug + Send + Sync {
    /// Resolve one stable attachment-search response batch.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested attachment-search payload cannot be
    /// materialized for the current runtime host.
    async fn attachment_search_batch(
        &self,
        query_text: &str,
        limit: usize,
        ext_filters: &std::collections::HashSet<String>,
        kind_filters: &std::collections::HashSet<String>,
        case_sensitive: bool,
    ) -> Result<LanceRecordBatch, String>;
}

/// Runtime-owned provider contract for stable AST-search Flight reads.
#[async_trait]
pub trait AstSearchFlightRouteProvider: std::fmt::Debug + Send + Sync {
    /// Resolve one stable AST-search response batch.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested AST-search payload cannot be
    /// materialized for the current runtime host.
    async fn ast_search_batch(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<LanceRecordBatch, String>;
}

/// Runtime-owned provider contract for stable markdown analysis Flight reads.
#[async_trait]
pub trait MarkdownAnalysisFlightRouteProvider: std::fmt::Debug + Send + Sync {
    /// Resolve one stable markdown analysis response batch.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested markdown analysis payload cannot be
    /// materialized for the current runtime host.
    async fn markdown_analysis_batch(&self, path: &str) -> Result<LanceRecordBatch, String>;
}

/// Runtime-owned provider contract for stable code-AST analysis Flight reads.
#[async_trait]
pub trait CodeAstAnalysisFlightRouteProvider: std::fmt::Debug + Send + Sync {
    /// Resolve one stable code-AST analysis response batch.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested code-AST analysis payload cannot be
    /// materialized for the current runtime host.
    async fn code_ast_analysis_batch(
        &self,
        path: &str,
        repo_id: &str,
        line_hint: Option<usize>,
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
    weights: RerankScoreWeights,
}

impl RerankFlightRouteHandler {
    /// Create one rerank Flight route handler.
    ///
    /// # Errors
    ///
    /// Returns an error when the expected embedding dimension is zero.
    pub fn new(expected_dimension: usize) -> Result<Self, String> {
        Self::new_with_weights(expected_dimension, RerankScoreWeights::default())
    }

    /// Create one rerank Flight route handler with explicit runtime-owned
    /// score weights.
    ///
    /// # Errors
    ///
    /// Returns an error when the expected embedding dimension is zero or when
    /// the runtime weights are invalid.
    pub fn new_with_weights(
        expected_dimension: usize,
        weights: RerankScoreWeights,
    ) -> Result<Self, String> {
        if expected_dimension == 0 {
            return Err("rerank route expected_dimension must be greater than zero".to_string());
        }
        Ok(Self {
            expected_dimension,
            weights: RerankScoreWeights::new(weights.vector_weight, weights.semantic_weight)?,
        })
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
        top_k: Option<usize>,
        min_final_score: Option<f64>,
    ) -> Result<LanceRecordBatch, String> {
        let mut scored_candidates = Vec::new();
        for batch in request_batches {
            scored_candidates.extend(score_rerank_request_batch_with_weights(
                batch,
                self.expected_dimension,
                self.weights,
            )?);
        }

        if scored_candidates.is_empty() {
            return Err("rerank request batches must contain at least one row".to_string());
        }

        if let Some(threshold) = min_final_score {
            scored_candidates.retain(|candidate| candidate.final_score >= threshold);
        }

        scored_candidates.sort_by(|left, right| {
            right
                .final_score
                .partial_cmp(&left.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.doc_id.cmp(&right.doc_id))
        });
        if let Some(limit) = top_k {
            scored_candidates.truncate(limit);
        }

        let doc_ids = scored_candidates
            .iter()
            .map(|candidate| candidate.doc_id.clone())
            .collect::<Vec<_>>();
        let vector_scores = scored_candidates
            .iter()
            .map(|candidate| candidate.vector_score)
            .collect::<Vec<_>>();
        let semantic_scores = scored_candidates
            .iter()
            .map(|candidate| candidate.semantic_score)
            .collect::<Vec<_>>();
        let final_scores = scored_candidates
            .iter()
            .map(|candidate| candidate.final_score)
            .collect::<Vec<_>>();
        let ranks = (1..=i32::try_from(scored_candidates.len())
            .map_err(|error| format!("failed to represent rerank response rank range: {error}"))?)
            .collect::<Vec<_>>();

        LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new(RERANK_RESPONSE_DOC_ID_COLUMN, LanceDataType::Utf8, false),
                LanceField::new(
                    RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
                    LanceDataType::Float64,
                    false,
                ),
                LanceField::new(
                    RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
                    LanceDataType::Float64,
                    false,
                ),
                LanceField::new(
                    RERANK_RESPONSE_FINAL_SCORE_COLUMN,
                    LanceDataType::Float64,
                    false,
                ),
                LanceField::new(RERANK_RESPONSE_RANK_COLUMN, LanceDataType::Int32, false),
            ])),
            vec![
                Arc::new(LanceStringArray::from(doc_ids)),
                Arc::new(LanceFloat64Array::from(vector_scores)),
                Arc::new(LanceFloat64Array::from(semantic_scores)),
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
    search_provider: Option<Arc<dyn SearchFlightRouteProvider>>,
    attachment_search_provider: Option<Arc<dyn AttachmentSearchFlightRouteProvider>>,
    ast_search_provider: Option<Arc<dyn AstSearchFlightRouteProvider>>,
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
        markdown_analysis_provider: Option<Arc<dyn MarkdownAnalysisFlightRouteProvider>>,
        code_ast_analysis_provider: Option<Arc<dyn CodeAstAnalysisFlightRouteProvider>>,
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
                .map(FlightRoutePayload::new)
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
                .map(FlightRoutePayload::new)
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

fn validate_rerank_top_k_header(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<Option<usize>, Status> {
    let Some(raw_value) = metadata.get(WENDAO_RERANK_TOP_K_HEADER) else {
        return Ok(None);
    };
    let top_k = raw_value.to_str().unwrap_or_default();
    let parsed_top_k = top_k.parse::<usize>().map_err(|_| {
        Status::invalid_argument(format!(
            "invalid rerank top_k header `{WENDAO_RERANK_TOP_K_HEADER}`: {top_k}"
        ))
    })?;
    if parsed_top_k == 0 {
        return Err(Status::invalid_argument(format!(
            "rerank top_k header `{WENDAO_RERANK_TOP_K_HEADER}` must be greater than zero"
        )));
    }
    Ok(Some(parsed_top_k))
}

fn validate_rerank_min_final_score_header(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<Option<f64>, Status> {
    let Some(raw_value) = metadata.get(WENDAO_RERANK_MIN_FINAL_SCORE_HEADER) else {
        return Ok(None);
    };
    let min_final_score = raw_value.to_str().unwrap_or_default();
    let parsed_min_final_score = min_final_score.parse::<f64>().map_err(|_| {
        Status::invalid_argument(format!(
            "invalid rerank min_final_score header `{WENDAO_RERANK_MIN_FINAL_SCORE_HEADER}`: {min_final_score}"
        ))
    })?;
    if !parsed_min_final_score.is_finite() {
        return Err(Status::invalid_argument(format!(
            "rerank min_final_score header `{WENDAO_RERANK_MIN_FINAL_SCORE_HEADER}` must be finite"
        )));
    }
    if !(0.0..=1.0).contains(&parsed_min_final_score) {
        return Err(Status::invalid_argument(format!(
            "rerank min_final_score header `{WENDAO_RERANK_MIN_FINAL_SCORE_HEADER}` must stay within inclusive range [0.0, 1.0]"
        )));
    }
    Ok(Some(parsed_min_final_score))
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
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let path_prefix_values = metadata
        .get(WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let title_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let tag_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let filename_filter_values = metadata
        .get(WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER)
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

fn validate_search_request_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<(String, usize, Option<String>, Option<String>), Status> {
    let query_text = metadata
        .get(WENDAO_SEARCH_QUERY_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let limit = metadata
        .get(WENDAO_SEARCH_LIMIT_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let parsed_limit = limit.parse::<usize>().map_err(|_| {
        Status::invalid_argument(format!(
            "invalid search limit header `{WENDAO_SEARCH_LIMIT_HEADER}`: {limit}"
        ))
    })?;
    let intent = metadata
        .get(WENDAO_SEARCH_INTENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let repo_hint = metadata
        .get(WENDAO_SEARCH_REPO_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    validate_repo_search_request(query_text.as_str(), parsed_limit, &[], &[], &[], &[], &[])
        .map_err(Status::invalid_argument)?;
    Ok((query_text, parsed_limit, intent, repo_hint))
}

fn validate_markdown_analysis_request_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<String, Status> {
    let path = metadata
        .get(WENDAO_ANALYSIS_PATH_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    validate_markdown_analysis_request(path.as_str()).map_err(Status::invalid_argument)?;
    Ok(path)
}

fn validate_code_ast_analysis_request_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<(String, String, Option<usize>), Status> {
    let path = metadata
        .get(WENDAO_ANALYSIS_PATH_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let repo_id = metadata
        .get(WENDAO_ANALYSIS_REPO_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let line_hint = match metadata.get(WENDAO_ANALYSIS_LINE_HEADER) {
        Some(raw_value) => {
            let line_hint = raw_value.to_str().unwrap_or_default();
            Some(line_hint.parse::<usize>().map_err(|_| {
                Status::invalid_argument(format!(
                    "invalid analysis line header `{WENDAO_ANALYSIS_LINE_HEADER}`: {line_hint}"
                ))
            })?)
        }
        None => None,
    };
    validate_code_ast_analysis_request(path.as_str(), repo_id.as_str(), line_hint)
        .map_err(Status::invalid_argument)?;
    Ok((path, repo_id, line_hint))
}

fn validate_attachment_search_request_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<
    (
        String,
        usize,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
        bool,
    ),
    Status,
> {
    let query_text = metadata
        .get(WENDAO_SEARCH_QUERY_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let limit = metadata
        .get(WENDAO_SEARCH_LIMIT_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let parsed_limit = limit.parse::<usize>().map_err(|_| {
        Status::invalid_argument(format!(
            "invalid search limit header `{WENDAO_SEARCH_LIMIT_HEADER}`: {limit}"
        ))
    })?;
    let ext_filter_values = metadata
        .get(WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let kind_filter_values = metadata
        .get(WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .split(',')
        .filter(|value| {
            !value.is_empty() || metadata.contains_key(WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER)
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let case_sensitive = metadata
        .get(WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("false")
        .parse::<bool>()
        .map_err(|_| {
            Status::invalid_argument(format!(
                "invalid attachment-search case_sensitive header `{WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER}`"
            ))
        })?;
    validate_attachment_search_request(
        query_text.as_str(),
        parsed_limit,
        &ext_filter_values,
        &kind_filter_values,
    )
    .map_err(Status::invalid_argument)?;
    let ext_filters = ext_filter_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let kind_filters = kind_filter_values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::HashSet<_>>();
    Ok((
        query_text,
        parsed_limit,
        ext_filters,
        kind_filters,
        case_sensitive,
    ))
}

fn descriptor_route(descriptor: &FlightDescriptor) -> Result<String, Status> {
    let actual_path = descriptor
        .path
        .iter()
        .map(|segment| String::from_utf8_lossy(segment.as_ref()).into_owned())
        .collect::<Vec<_>>();
    normalize_flight_route(format!("/{}", actual_path.join("/"))).map_err(Status::invalid_argument)
}

fn ticket_route(ticket: &Ticket) -> Result<String, Status> {
    let route = String::from_utf8(ticket.ticket.to_vec())
        .map_err(|error| Status::invalid_argument(format!("invalid ticket bytes: {error}")))?;
    normalize_flight_route(route).map_err(Status::invalid_argument)
}

fn join_sorted_set(values: &std::collections::HashSet<String>) -> String {
    let mut sorted = values.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted.join(",")
}

fn is_search_family_route(route: &str) -> bool {
    matches!(
        route,
        SEARCH_INTENT_ROUTE
            | SEARCH_KNOWLEDGE_ROUTE
            | SEARCH_REFERENCES_ROUTE
            | SEARCH_SYMBOLS_ROUTE
    )
}

#[cfg(test)]
mod tests {
    use super::{
        AstSearchFlightRouteProvider, AttachmentSearchFlightRouteProvider,
        CodeAstAnalysisFlightRouteProvider, MarkdownAnalysisFlightRouteProvider,
        RepoSearchFlightRouteProvider, RerankFlightRouteHandler, RerankScoreWeights,
        SearchFlightRouteProvider, SearchFlightRouteResponse, WENDAO_RERANK_TOP_K_HEADER,
        WendaoFlightService, is_search_family_route, validate_attachment_search_request_metadata,
        validate_code_ast_analysis_request_metadata, validate_markdown_analysis_request_metadata,
        validate_rerank_top_k_header, validate_search_request_metadata,
    };
    use arrow_array::types::Float32Type;
    use arrow_array::{FixedSizeListArray, Float32Array, StringArray};
    use arrow_flight::flight_service_server::FlightService;
    use arrow_flight::{FlightDescriptor, Ticket};
    use async_trait::async_trait;
    use futures::StreamExt;
    use std::sync::{Arc, Mutex};
    use tonic::Request;
    use tonic::metadata::{MetadataMap, MetadataValue};
    use xiuxian_vector::{
        LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema,
    };

    use crate::transport::{
        ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, SEARCH_AST_ROUTE,
        SEARCH_ATTACHMENTS_ROUTE, SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE,
        WENDAO_ANALYSIS_LINE_HEADER, WENDAO_ANALYSIS_PATH_HEADER, WENDAO_ANALYSIS_REPO_HEADER,
        WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
        WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER, WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
        WENDAO_SCHEMA_VERSION_HEADER, WENDAO_SEARCH_INTENT_HEADER, WENDAO_SEARCH_LIMIT_HEADER,
        WENDAO_SEARCH_QUERY_HEADER, WENDAO_SEARCH_REPO_HEADER, flight_descriptor_path,
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
            .handle_exchange_batches(std::slice::from_ref(&request_batch), None, None)
            .expect("rerank route handler should score request batches");

        let doc_ids = response
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("doc_id should decode as Utf8");
        let vector_scores = response
            .column_by_name("vector_score")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Float64Array>())
            .expect("vector_score should decode as Float64");
        let semantic_scores = response
            .column_by_name("semantic_score")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Float64Array>())
            .expect("semantic_score should decode as Float64");
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
        assert!((vector_scores.value(0) - 0.5).abs() < 1e-6);
        assert!((vector_scores.value(1) - 0.8).abs() < 1e-6);
        assert!((semantic_scores.value(0) - 1.0).abs() < 1e-6);
        assert!((semantic_scores.value(1) - 0.5).abs() < 1e-6);
        assert!((final_scores.value(0) - 0.8).abs() < 1e-6);
        assert!((final_scores.value(1) - 0.62).abs() < 1e-6);
        assert_eq!(ranks.value(0), 1);
        assert_eq!(ranks.value(1), 2);
    }

    #[test]
    fn rerank_route_handler_respects_runtime_weight_policy() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let request_batch = RecordBatch::try_new(
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
        .expect("request batch should build");
        let handler = RerankFlightRouteHandler::new_with_weights(
            3,
            RerankScoreWeights::new(0.9, 0.1).expect("weights should validate"),
        )
        .expect("handler should build");

        let response = handler
            .handle_exchange_batches(std::slice::from_ref(&request_batch), None, None)
            .expect("rerank route handler should score request batches");

        let doc_ids = response
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("doc_id should decode as Utf8");
        let final_scores = response
            .column_by_name("final_score")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Float64Array>())
            .expect("final_score should decode as Float64");

        assert_eq!(doc_ids.value(0), "doc-1");
        assert_eq!(doc_ids.value(1), "doc-0");
        assert!((final_scores.value(0) - 0.77).abs() < 1e-6);
        assert!((final_scores.value(1) - 0.55).abs() < 1e-6);
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
    fn rerank_route_handler_applies_top_k_after_scoring() {
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
            .handle_exchange_batches(std::slice::from_ref(&request_batch), Some(1), None)
            .expect("rerank route handler should truncate scored request batches");

        let doc_ids = response
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("doc_id should decode as Utf8");
        let ranks = response
            .column_by_name("rank")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Int32Array>())
            .expect("rank should decode as Int32");

        assert_eq!(response.num_rows(), 1);
        assert_eq!(doc_ids.value(0), "doc-0");
        assert_eq!(ranks.value(0), 1);
    }

    #[test]
    fn rerank_route_handler_preserves_full_result_when_top_k_exceeds_candidate_count() {
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
            .handle_exchange_batches(std::slice::from_ref(&request_batch), Some(10), None)
            .expect("rerank route handler should preserve all scored request batches");

        let doc_ids = response
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("doc_id should decode as Utf8");
        let ranks = response
            .column_by_name("rank")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Int32Array>())
            .expect("rank should decode as Int32");

        assert_eq!(response.num_rows(), 2);
        assert_eq!(doc_ids.value(0), "doc-0");
        assert_eq!(doc_ids.value(1), "doc-1");
        assert_eq!(ranks.value(0), 1);
        assert_eq!(ranks.value(1), 2);
    }

    #[test]
    fn rerank_route_handler_preserves_full_result_when_top_k_matches_candidate_count() {
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
            .handle_exchange_batches(std::slice::from_ref(&request_batch), Some(2), None)
            .expect("rerank route handler should preserve all scored request batches");

        let doc_ids = response
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .expect("doc_id should decode as Utf8");
        let ranks = response
            .column_by_name("rank")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Int32Array>())
            .expect("rank should decode as Int32");

        assert_eq!(response.num_rows(), 2);
        assert_eq!(doc_ids.value(0), "doc-0");
        assert_eq!(doc_ids.value(1), "doc-1");
        assert_eq!(ranks.value(0), 1);
        assert_eq!(ranks.value(1), 2);
    }

    #[test]
    fn validate_rerank_top_k_header_rejects_zero() {
        let mut metadata = MetadataMap::new();
        metadata.insert(
            WENDAO_RERANK_TOP_K_HEADER,
            "0".parse().expect("metadata should parse"),
        );

        let error =
            validate_rerank_top_k_header(&metadata).expect_err("zero rerank top_k should fail");

        assert_eq!(
            error.message(),
            "rerank top_k header `x-wendao-rerank-top-k` must be greater than zero"
        );
    }

    #[test]
    fn validate_rerank_top_k_header_rejects_non_numeric_values() {
        let mut metadata = MetadataMap::new();
        metadata.insert(
            WENDAO_RERANK_TOP_K_HEADER,
            "abc".parse().expect("metadata should parse"),
        );

        let error = validate_rerank_top_k_header(&metadata)
            .expect_err("non-numeric rerank top_k should fail");

        assert_eq!(
            error.message(),
            "invalid rerank top_k header `x-wendao-rerank-top-k`: abc"
        );
    }

    #[test]
    fn validate_rerank_top_k_header_rejects_blank_values() {
        let mut metadata = MetadataMap::new();
        metadata.insert(
            WENDAO_RERANK_TOP_K_HEADER,
            "".parse().expect("metadata should parse"),
        );

        let error =
            validate_rerank_top_k_header(&metadata).expect_err("blank rerank top_k should fail");

        assert_eq!(
            error.message(),
            "invalid rerank top_k header `x-wendao-rerank-top-k`: "
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

    #[test]
    fn validate_search_request_metadata_accepts_stable_request() {
        let metadata = build_search_metadata("semantic-route", "7");

        let (query_text, limit, intent, repo_hint) = validate_search_request_metadata(&metadata)
            .expect("stable search-family metadata should validate");

        assert_eq!(query_text, "semantic-route");
        assert_eq!(limit, 7);
        assert_eq!(intent, None);
        assert_eq!(repo_hint, None);
    }

    #[test]
    fn validate_search_request_metadata_accepts_intent_and_repo_hints() {
        let mut metadata = MetadataMap::new();
        populate_schema_and_search_headers_with_hints(
            &mut metadata,
            "semantic-route",
            "7",
            Some("code_search"),
            Some("gateway-sync"),
        );

        let (query_text, limit, intent, repo_hint) = validate_search_request_metadata(&metadata)
            .expect("search-family metadata with hints should validate");

        assert_eq!(query_text, "semantic-route");
        assert_eq!(limit, 7);
        assert_eq!(intent.as_deref(), Some("code_search"));
        assert_eq!(repo_hint.as_deref(), Some("gateway-sync"));
    }

    #[test]
    fn validate_search_request_metadata_rejects_blank_query_text() {
        let metadata = build_search_metadata("", "7");

        let error = validate_search_request_metadata(&metadata)
            .expect_err("blank search-family query text should fail");

        assert_eq!(error.message(), "repo search query text must not be blank");
    }

    #[test]
    fn validate_search_request_metadata_rejects_zero_limit() {
        let metadata = build_search_metadata("semantic-route", "0");

        let error = validate_search_request_metadata(&metadata)
            .expect_err("zero search-family limit should fail");

        assert_eq!(
            error.message(),
            "repo search limit must be greater than zero"
        );
    }

    #[test]
    fn validate_markdown_analysis_request_metadata_accepts_stable_request() {
        let metadata = build_markdown_analysis_metadata("docs/analysis.md");

        let path = validate_markdown_analysis_request_metadata(&metadata)
            .expect("stable markdown analysis metadata should validate");

        assert_eq!(path, "docs/analysis.md");
    }

    #[test]
    fn validate_markdown_analysis_request_metadata_rejects_blank_path() {
        let metadata = build_markdown_analysis_metadata("   ");

        let error = validate_markdown_analysis_request_metadata(&metadata)
            .expect_err("blank markdown analysis path should fail");

        assert_eq!(error.message(), "markdown analysis path must not be blank");
    }

    #[test]
    fn validate_code_ast_analysis_request_metadata_accepts_stable_request() {
        let metadata = build_code_ast_analysis_metadata("src/lib.jl", "demo", Some("7"));

        let (path, repo_id, line_hint) = validate_code_ast_analysis_request_metadata(&metadata)
            .expect("stable code-AST analysis metadata should validate");

        assert_eq!(path, "src/lib.jl");
        assert_eq!(repo_id, "demo");
        assert_eq!(line_hint, Some(7));
    }

    #[test]
    fn validate_code_ast_analysis_request_metadata_rejects_blank_repo() {
        let metadata = build_code_ast_analysis_metadata("src/lib.jl", "   ", None);

        let error = validate_code_ast_analysis_request_metadata(&metadata)
            .expect_err("blank code-AST repo should fail");

        assert_eq!(error.message(), "code AST analysis repo must not be blank");
    }

    #[test]
    fn validate_code_ast_analysis_request_metadata_rejects_non_numeric_line_hint() {
        let metadata = build_code_ast_analysis_metadata("src/lib.jl", "demo", Some("abc"));

        let error = validate_code_ast_analysis_request_metadata(&metadata)
            .expect_err("non-numeric code-AST line hint should fail");

        assert_eq!(
            error.message(),
            "invalid analysis line header `x-wendao-analysis-line`: abc"
        );
    }

    #[test]
    fn validate_attachment_search_request_metadata_accepts_stable_request() {
        let metadata = build_attachment_search_metadata(
            "image",
            "5",
            Some("png,jpg"),
            Some("image,screenshot"),
            Some("true"),
        );

        let (query_text, limit, ext_filters, kind_filters, case_sensitive) =
            validate_attachment_search_request_metadata(&metadata)
                .expect("stable attachment-search metadata should validate");

        assert_eq!(query_text, "image");
        assert_eq!(limit, 5);
        assert!(ext_filters.contains("png"));
        assert!(ext_filters.contains("jpg"));
        assert!(kind_filters.contains("image"));
        assert!(kind_filters.contains("screenshot"));
        assert!(case_sensitive);
    }

    #[test]
    fn validate_attachment_search_request_metadata_rejects_blank_extension_filters() {
        let metadata =
            build_attachment_search_metadata("image", "5", Some("png, "), Some("image"), None);

        let error = validate_attachment_search_request_metadata(&metadata)
            .expect_err("blank extension filter should fail");

        assert_eq!(
            error.message(),
            "attachment search extension filters must not contain blank values"
        );
    }

    #[test]
    fn search_family_route_matcher_accepts_semantic_business_routes() {
        assert!(is_search_family_route(SEARCH_INTENT_ROUTE));
        assert!(is_search_family_route(SEARCH_KNOWLEDGE_ROUTE));
        assert!(!is_search_family_route(SEARCH_ATTACHMENTS_ROUTE));
        assert!(!is_search_family_route(SEARCH_AST_ROUTE));
        assert!(!is_search_family_route(ANALYSIS_MARKDOWN_ROUTE));
        assert!(!is_search_family_route(ANALYSIS_CODE_AST_ROUTE));
    }

    #[tokio::test]
    async fn wendao_flight_service_get_flight_info_uses_search_family_provider() {
        let provider = Arc::new(RecordingSearchProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(provider.clone()),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with search-family provider");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_INTENT_ROUTE).expect("descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_search_headers(request.metadata_mut(), "semantic-route", "4");

        let response = service
            .get_flight_info(request)
            .await
            .expect("search-family route should resolve through the pluggable provider");
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("search-family route should emit one ticket");
        let app_metadata: serde_json::Value =
            serde_json::from_slice(&flight_info.app_metadata).expect("app_metadata should decode");

        assert_eq!(ticket, SEARCH_INTENT_ROUTE);
        assert_eq!(app_metadata["query"], "semantic-route");
        assert_eq!(app_metadata["hitCount"], 1);
        assert_eq!(provider.call_count(), 1);
        assert_eq!(
            provider.recorded_request(),
            Some((
                SEARCH_INTENT_ROUTE.to_string(),
                "semantic-route".to_string(),
                4,
                None,
                None,
            ))
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_do_get_reuses_search_family_provider_batch() {
        let provider = Arc::new(RecordingSearchProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(provider.clone()),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with search-family provider");
        let mut request = Request::new(Ticket::new(SEARCH_INTENT_ROUTE.to_string()));
        populate_schema_and_search_headers(request.metadata_mut(), "semantic-route", "2");

        let response = service
            .do_get(request)
            .await
            .expect("search-family route should stream through the pluggable provider");
        let frames = response.into_inner().collect::<Vec<_>>().await;

        assert!(!frames.is_empty());
        assert_eq!(provider.call_count(), 1);
        assert_eq!(
            provider.recorded_request(),
            Some((
                SEARCH_INTENT_ROUTE.to_string(),
                "semantic-route".to_string(),
                2,
                None,
                None,
            ))
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_do_get_reuses_cached_search_family_payload_after_get_flight_info()
     {
        let provider = Arc::new(RecordingSearchProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(provider.clone()),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with search-family provider");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_INTENT_ROUTE).expect("descriptor path should build"),
        );
        let mut flight_info_request = Request::new(descriptor);
        populate_schema_and_search_headers(
            flight_info_request.metadata_mut(),
            "semantic-route",
            "5",
        );
        let flight_info = service
            .get_flight_info(flight_info_request)
            .await
            .expect("search-family route should resolve through the pluggable provider")
            .into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.clone())
            .expect("search-family route should emit one ticket");

        let mut do_get_request = Request::new(ticket);
        populate_schema_and_search_headers(do_get_request.metadata_mut(), "semantic-route", "5");
        let response = service
            .do_get(do_get_request)
            .await
            .expect("search-family route should reuse the cached payload");
        let frames = response.into_inner().collect::<Vec<_>>().await;

        assert!(!frames.is_empty());
        assert_eq!(provider.call_count(), 1);
    }

    #[tokio::test]
    async fn wendao_flight_service_rejects_unconfigured_search_family_route() {
        let service = WendaoFlightService::new_with_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_INTENT_ROUTE).expect("descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_search_headers(request.metadata_mut(), "semantic-route", "4");

        let error = service
            .get_flight_info(request)
            .await
            .expect_err("unconfigured search-family route should fail");

        assert_eq!(error.code(), tonic::Code::Unimplemented);
        assert_eq!(
            error.message(),
            "search Flight route `/search/intent` is not configured for this runtime host"
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_get_flight_info_uses_attachment_search_provider() {
        let provider = Arc::new(RecordingAttachmentSearchProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            None,
            Some(provider.clone()),
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with attachment-search provider");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_ATTACHMENTS_ROUTE)
                .expect("attachment descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_attachment_search_headers(
            request.metadata_mut(),
            "image",
            "4",
            Some("png,jpg"),
            Some("image,screenshot"),
            Some("true"),
        );

        let response = service
            .get_flight_info(request)
            .await
            .expect("attachment-search route should resolve through the pluggable provider");
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("attachment-search route should emit one ticket");

        assert_eq!(ticket, SEARCH_ATTACHMENTS_ROUTE);
        assert_eq!(
            provider.recorded_request(),
            Some((
                "image".to_string(),
                4,
                vec!["jpg".to_string(), "png".to_string()],
                vec!["image".to_string(), "screenshot".to_string()],
                true,
            ))
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_rejects_unconfigured_attachment_search_route() {
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(Arc::new(RecordingSearchProvider::default())),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_ATTACHMENTS_ROUTE)
                .expect("attachment descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_attachment_search_headers(
            request.metadata_mut(),
            "image",
            "4",
            Some("png"),
            Some("image"),
            Some("false"),
        );

        let error = service
            .get_flight_info(request)
            .await
            .expect_err("unconfigured attachment-search route should fail");

        assert_eq!(error.code(), tonic::Code::Unimplemented);
        assert_eq!(
            error.message(),
            "attachment-search Flight route `/search/attachments` is not configured for this runtime host"
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_get_flight_info_uses_ast_search_provider() {
        let provider = Arc::new(RecordingAstSearchProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            None,
            None,
            Some(provider.clone()),
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with AST-search provider");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_AST_ROUTE).expect("AST descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_search_headers(request.metadata_mut(), "symbol", "6");

        let response = service
            .get_flight_info(request)
            .await
            .expect("AST route should resolve through the dedicated provider");
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("AST route should emit one ticket");

        assert_eq!(ticket, SEARCH_AST_ROUTE);
        assert_eq!(provider.recorded_request(), Some(("symbol".to_string(), 6)));
    }

    #[tokio::test]
    async fn wendao_flight_service_rejects_unconfigured_ast_search_route() {
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(Arc::new(RecordingSearchProvider::default())),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_AST_ROUTE).expect("AST descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_search_headers(request.metadata_mut(), "symbol", "6");

        let error = service
            .get_flight_info(request)
            .await
            .expect_err("unconfigured AST route should fail");

        assert_eq!(error.code(), tonic::Code::Unimplemented);
        assert_eq!(
            error.message(),
            "AST-search Flight route `/search/ast` is not configured for this runtime host"
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_get_flight_info_uses_markdown_analysis_provider() {
        let provider = Arc::new(RecordingMarkdownAnalysisProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            None,
            None,
            None,
            Some(provider.clone()),
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with markdown analysis provider");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_MARKDOWN_ROUTE)
                .expect("markdown analysis descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_markdown_analysis_headers(request.metadata_mut(), "docs/analysis.md");

        let response = service
            .get_flight_info(request)
            .await
            .expect("markdown analysis route should resolve through the dedicated provider");
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("markdown analysis route should emit one ticket");

        assert_eq!(ticket, ANALYSIS_MARKDOWN_ROUTE);
        assert_eq!(
            provider.recorded_request(),
            Some("docs/analysis.md".to_string())
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_get_flight_info_uses_code_ast_analysis_provider() {
        let provider = Arc::new(RecordingCodeAstAnalysisProvider::default());
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            None,
            None,
            None,
            None,
            Some(provider.clone()),
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build with code-AST analysis provider");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_CODE_AST_ROUTE)
                .expect("code-AST analysis descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_code_ast_analysis_headers(
            request.metadata_mut(),
            "src/lib.jl",
            "demo",
            Some("7"),
        );

        let response = service
            .get_flight_info(request)
            .await
            .expect("code-AST analysis route should resolve through the dedicated provider");
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("code-AST analysis route should emit one ticket");

        assert_eq!(ticket, ANALYSIS_CODE_AST_ROUTE);
        assert_eq!(
            provider.recorded_request(),
            Some(("src/lib.jl".to_string(), "demo".to_string(), Some(7)))
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_rejects_unconfigured_markdown_analysis_route() {
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(Arc::new(RecordingSearchProvider::default())),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_MARKDOWN_ROUTE)
                .expect("markdown analysis descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_markdown_analysis_headers(request.metadata_mut(), "docs/analysis.md");

        let error = service
            .get_flight_info(request)
            .await
            .expect_err("unconfigured markdown analysis route should fail");

        assert_eq!(error.code(), tonic::Code::Unimplemented);
        assert_eq!(
            error.message(),
            "markdown analysis Flight route `/analysis/markdown` is not configured for this runtime host"
        );
    }

    #[tokio::test]
    async fn wendao_flight_service_rejects_unconfigured_code_ast_analysis_route() {
        let service = WendaoFlightService::new_with_route_providers(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Some(Arc::new(RecordingSearchProvider::default())),
            None,
            None,
            None,
            None,
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_CODE_AST_ROUTE)
                .expect("code-AST analysis descriptor path should build"),
        );
        let mut request = Request::new(descriptor);
        populate_schema_and_code_ast_analysis_headers(
            request.metadata_mut(),
            "src/lib.jl",
            "demo",
            Some("7"),
        );

        let error = service
            .get_flight_info(request)
            .await
            .expect_err("unconfigured code-AST analysis route should fail");

        assert_eq!(error.code(), tonic::Code::Unimplemented);
        assert_eq!(
            error.message(),
            "code-AST analysis Flight route `/analysis/code-ast` is not configured for this runtime host"
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

    #[derive(Debug, Default)]
    struct RecordingSearchProvider {
        request: std::sync::Mutex<Option<(String, String, usize, Option<String>, Option<String>)>>,
        call_count: std::sync::Mutex<usize>,
    }

    impl RecordingSearchProvider {
        fn recorded_request(
            &self,
        ) -> Option<(String, String, usize, Option<String>, Option<String>)> {
            self.request
                .lock()
                .expect("search-family provider record should lock")
                .clone()
        }

        fn call_count(&self) -> usize {
            *self
                .call_count
                .lock()
                .expect("search-family provider call count should lock")
        }
    }

    #[async_trait]
    impl SearchFlightRouteProvider for RecordingSearchProvider {
        async fn search_batch(
            &self,
            route: &str,
            query_text: &str,
            limit: usize,
            intent: Option<&str>,
            repo_hint: Option<&str>,
        ) -> Result<SearchFlightRouteResponse, String> {
            *self
                .request
                .lock()
                .expect("search-family provider record should lock") = Some((
                route.to_string(),
                query_text.to_string(),
                limit,
                intent.map(ToString::to_string),
                repo_hint.map(ToString::to_string),
            ));
            *self
                .call_count
                .lock()
                .expect("search-family provider call count should lock") += 1;
            let batch = LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("doc_id", LanceDataType::Utf8, false),
                    LanceField::new("route", LanceDataType::Utf8, false),
                    LanceField::new("query_text", LanceDataType::Utf8, false),
                    LanceField::new("score", LanceDataType::Float64, false),
                ])),
                vec![
                    Arc::new(StringArray::from(vec![format!(
                        "{route}:{query_text}:{limit}"
                    )])),
                    Arc::new(StringArray::from(vec![route.to_string()])),
                    Arc::new(StringArray::from(vec![query_text.to_string()])),
                    Arc::new(LanceFloat64Array::from(vec![0.99_f64])),
                ],
            )
            .map_err(|error| error.to_string())?;
            Ok(SearchFlightRouteResponse::new(batch).with_app_metadata(
                serde_json::json!({
                    "query": query_text,
                    "hitCount": 1,
                    "selectedMode": route,
                    "intent": intent,
                    "repoHint": repo_hint,
                })
                .to_string()
                .into_bytes(),
            ))
        }
    }

    #[derive(Debug, Default)]
    struct RecordingAttachmentSearchProvider {
        request: Mutex<Option<(String, usize, Vec<String>, Vec<String>, bool)>>,
    }

    impl RecordingAttachmentSearchProvider {
        fn recorded_request(&self) -> Option<(String, usize, Vec<String>, Vec<String>, bool)> {
            self.request
                .lock()
                .expect("attachment-search provider record should lock")
                .clone()
        }
    }

    #[async_trait]
    impl AttachmentSearchFlightRouteProvider for RecordingAttachmentSearchProvider {
        async fn attachment_search_batch(
            &self,
            query_text: &str,
            limit: usize,
            ext_filters: &std::collections::HashSet<String>,
            kind_filters: &std::collections::HashSet<String>,
            case_sensitive: bool,
        ) -> Result<LanceRecordBatch, String> {
            let mut ext_filters = ext_filters.iter().cloned().collect::<Vec<_>>();
            ext_filters.sort();
            let mut kind_filters = kind_filters.iter().cloned().collect::<Vec<_>>();
            kind_filters.sort();
            *self
                .request
                .lock()
                .expect("attachment-search provider record should lock") = Some((
                query_text.to_string(),
                limit,
                ext_filters,
                kind_filters,
                case_sensitive,
            ));
            LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("doc_id", LanceDataType::Utf8, false),
                    LanceField::new("query_text", LanceDataType::Utf8, false),
                    LanceField::new("score", LanceDataType::Float64, false),
                ])),
                vec![
                    Arc::new(StringArray::from(vec![format!(
                        "attachment:{query_text}:{limit}"
                    )])),
                    Arc::new(StringArray::from(vec![query_text.to_string()])),
                    Arc::new(LanceFloat64Array::from(vec![0.77_f64])),
                ],
            )
            .map_err(|error| error.to_string())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingAstSearchProvider {
        request: Mutex<Option<(String, usize)>>,
    }

    impl RecordingAstSearchProvider {
        fn recorded_request(&self) -> Option<(String, usize)> {
            self.request
                .lock()
                .expect("AST-search provider record should lock")
                .clone()
        }
    }

    #[async_trait]
    impl AstSearchFlightRouteProvider for RecordingAstSearchProvider {
        async fn ast_search_batch(
            &self,
            query_text: &str,
            limit: usize,
        ) -> Result<LanceRecordBatch, String> {
            *self
                .request
                .lock()
                .expect("AST-search provider record should lock") =
                Some((query_text.to_string(), limit));
            LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("doc_id", LanceDataType::Utf8, false),
                    LanceField::new("query_text", LanceDataType::Utf8, false),
                    LanceField::new("score", LanceDataType::Float64, false),
                ])),
                vec![
                    Arc::new(StringArray::from(vec![format!("ast:{query_text}:{limit}")])),
                    Arc::new(StringArray::from(vec![query_text.to_string()])),
                    Arc::new(LanceFloat64Array::from(vec![0.81_f64])),
                ],
            )
            .map_err(|error| error.to_string())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingMarkdownAnalysisProvider {
        request: Mutex<Option<String>>,
    }

    impl RecordingMarkdownAnalysisProvider {
        fn recorded_request(&self) -> Option<String> {
            self.request
                .lock()
                .expect("markdown analysis provider record should lock")
                .clone()
        }
    }

    #[async_trait]
    impl MarkdownAnalysisFlightRouteProvider for RecordingMarkdownAnalysisProvider {
        async fn markdown_analysis_batch(&self, path: &str) -> Result<LanceRecordBatch, String> {
            *self
                .request
                .lock()
                .expect("markdown analysis provider record should lock") = Some(path.to_string());
            LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("ownerId", LanceDataType::Utf8, false),
                    LanceField::new("chunkId", LanceDataType::Utf8, false),
                    LanceField::new("semanticType", LanceDataType::Utf8, false),
                ])),
                vec![
                    Arc::new(StringArray::from(vec![format!("markdown:{path}")])),
                    Arc::new(StringArray::from(vec!["chunk:0"])),
                    Arc::new(StringArray::from(vec!["section"])),
                ],
            )
            .map_err(|error| error.to_string())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingCodeAstAnalysisProvider {
        request: Mutex<Option<(String, String, Option<usize>)>>,
    }

    impl RecordingCodeAstAnalysisProvider {
        fn recorded_request(&self) -> Option<(String, String, Option<usize>)> {
            self.request
                .lock()
                .expect("code-AST analysis provider record should lock")
                .clone()
        }
    }

    #[async_trait]
    impl CodeAstAnalysisFlightRouteProvider for RecordingCodeAstAnalysisProvider {
        async fn code_ast_analysis_batch(
            &self,
            path: &str,
            repo_id: &str,
            line_hint: Option<usize>,
        ) -> Result<LanceRecordBatch, String> {
            *self
                .request
                .lock()
                .expect("code-AST analysis provider record should lock") =
                Some((path.to_string(), repo_id.to_string(), line_hint));
            LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("ownerId", LanceDataType::Utf8, false),
                    LanceField::new("chunkId", LanceDataType::Utf8, false),
                    LanceField::new("semanticType", LanceDataType::Utf8, false),
                ])),
                vec![
                    Arc::new(StringArray::from(vec![format!(
                        "code-ast:{repo_id}:{path}"
                    )])),
                    Arc::new(StringArray::from(vec!["chunk:0"])),
                    Arc::new(StringArray::from(vec!["declaration"])),
                ],
            )
            .map_err(|error| error.to_string())
        }
    }

    fn build_search_metadata(query_text: &str, limit: &str) -> MetadataMap {
        let mut metadata = MetadataMap::new();
        populate_schema_and_search_headers(&mut metadata, query_text, limit);
        metadata
    }

    fn build_markdown_analysis_metadata(path: &str) -> MetadataMap {
        let mut metadata = MetadataMap::new();
        populate_schema_and_markdown_analysis_headers(&mut metadata, path);
        metadata
    }

    fn build_code_ast_analysis_metadata(
        path: &str,
        repo_id: &str,
        line_hint: Option<&str>,
    ) -> MetadataMap {
        let mut metadata = MetadataMap::new();
        populate_schema_and_code_ast_analysis_headers(&mut metadata, path, repo_id, line_hint);
        metadata
    }

    fn build_attachment_search_metadata(
        query_text: &str,
        limit: &str,
        ext_filters: Option<&str>,
        kind_filters: Option<&str>,
        case_sensitive: Option<&str>,
    ) -> MetadataMap {
        let mut metadata = MetadataMap::new();
        populate_schema_and_attachment_search_headers(
            &mut metadata,
            query_text,
            limit,
            ext_filters,
            kind_filters,
            case_sensitive,
        );
        metadata
    }

    fn populate_schema_and_search_headers(
        metadata: &mut MetadataMap,
        query_text: &str,
        limit: &str,
    ) {
        populate_schema_and_search_headers_with_hints(metadata, query_text, limit, None, None);
    }

    fn populate_schema_and_search_headers_with_hints(
        metadata: &mut MetadataMap,
        query_text: &str,
        limit: &str,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            MetadataValue::try_from("v2").expect("schema version metadata should parse"),
        );
        metadata.insert(
            WENDAO_SEARCH_QUERY_HEADER,
            MetadataValue::try_from(query_text)
                .expect("search-family query text metadata should parse"),
        );
        metadata.insert(
            WENDAO_SEARCH_LIMIT_HEADER,
            MetadataValue::try_from(limit).expect("search-family limit metadata should parse"),
        );
        if let Some(intent) = intent {
            metadata.insert(
                WENDAO_SEARCH_INTENT_HEADER,
                MetadataValue::try_from(intent)
                    .expect("search-family intent metadata should parse"),
            );
        }
        if let Some(repo_hint) = repo_hint {
            metadata.insert(
                WENDAO_SEARCH_REPO_HEADER,
                MetadataValue::try_from(repo_hint)
                    .expect("search-family repo metadata should parse"),
            );
        }
    }

    fn populate_schema_and_attachment_search_headers(
        metadata: &mut MetadataMap,
        query_text: &str,
        limit: &str,
        ext_filters: Option<&str>,
        kind_filters: Option<&str>,
        case_sensitive: Option<&str>,
    ) {
        populate_schema_and_search_headers(metadata, query_text, limit);
        if let Some(ext_filters) = ext_filters {
            metadata.insert(
                WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER,
                MetadataValue::try_from(ext_filters)
                    .expect("attachment-search ext filters metadata should parse"),
            );
        }
        if let Some(kind_filters) = kind_filters {
            metadata.insert(
                WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
                MetadataValue::try_from(kind_filters)
                    .expect("attachment-search kind filters metadata should parse"),
            );
        }
        if let Some(case_sensitive) = case_sensitive {
            metadata.insert(
                WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
                MetadataValue::try_from(case_sensitive)
                    .expect("attachment-search case_sensitive metadata should parse"),
            );
        }
    }

    fn populate_schema_and_markdown_analysis_headers(metadata: &mut MetadataMap, path: &str) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            MetadataValue::try_from("v2").expect("schema version metadata should parse"),
        );
        metadata.insert(
            WENDAO_ANALYSIS_PATH_HEADER,
            MetadataValue::try_from(path).expect("analysis path metadata should parse"),
        );
    }

    fn populate_schema_and_code_ast_analysis_headers(
        metadata: &mut MetadataMap,
        path: &str,
        repo_id: &str,
        line_hint: Option<&str>,
    ) {
        populate_schema_and_markdown_analysis_headers(metadata, path);
        metadata.insert(
            WENDAO_ANALYSIS_REPO_HEADER,
            MetadataValue::try_from(repo_id).expect("analysis repo metadata should parse"),
        );
        if let Some(line_hint) = line_hint {
            metadata.insert(
                WENDAO_ANALYSIS_LINE_HEADER,
                MetadataValue::try_from(line_hint).expect("analysis line metadata should parse"),
            );
        }
    }

    #[test]
    fn wendao_flight_service_accepts_pluggable_repo_search_provider() {
        let service = WendaoFlightService::new_with_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            3,
            RerankScoreWeights::default(),
        )
        .expect("service should build from a pluggable repo-search provider");

        assert_eq!(service.expected_schema_version, "v2");
    }
}
