use super::request_metadata::{
    is_search_family_route, validate_attachment_search_request_metadata,
    validate_autocomplete_request_metadata, validate_code_ast_analysis_request_metadata,
    validate_definition_request_metadata, validate_graph_neighbors_request_metadata,
    validate_markdown_analysis_request_metadata, validate_rerank_top_k_header,
    validate_search_request_metadata, validate_vfs_resolve_request_metadata,
};
use super::{
    AnalysisFlightRouteResponse, AstSearchFlightRouteProvider, AttachmentSearchFlightRouteProvider,
    AutocompleteFlightRouteProvider, AutocompleteFlightRouteResponse,
    CodeAstAnalysisFlightRouteProvider, DefinitionFlightRouteProvider,
    DefinitionFlightRouteResponse, GraphNeighborsFlightRouteProvider,
    GraphNeighborsFlightRouteResponse, MarkdownAnalysisFlightRouteProvider,
    RepoSearchFlightRouteProvider, RerankFlightRouteHandler, SearchFlightRouteProvider,
    SearchFlightRouteResponse, VfsResolveFlightRouteProvider, VfsResolveFlightRouteResponse,
    WendaoFlightService,
};
use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Float32Array, StringArray};
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::{FlightData, FlightDescriptor, Ticket};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use tonic::Request;
use tonic::metadata::{MetadataMap, MetadataValue};
use xiuxian_vector::{
    LanceBooleanArray, LanceDataType, LanceField, LanceFloat64Array, LanceInt32Array,
    LanceRecordBatch, LanceSchema,
};

use crate::transport::query_contract::WENDAO_RERANK_TOP_K_HEADER;
use crate::transport::{
    ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, GRAPH_NEIGHBORS_ROUTE, RerankScoreWeights,
    SEARCH_AST_ROUTE, SEARCH_ATTACHMENTS_ROUTE, SEARCH_AUTOCOMPLETE_ROUTE, SEARCH_DEFINITION_ROUTE,
    SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE, VFS_RESOLVE_ROUTE, WENDAO_ANALYSIS_LINE_HEADER,
    WENDAO_ANALYSIS_PATH_HEADER, WENDAO_ANALYSIS_REPO_HEADER,
    WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER, WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER,
    WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER, WENDAO_AUTOCOMPLETE_LIMIT_HEADER,
    WENDAO_AUTOCOMPLETE_PREFIX_HEADER, WENDAO_DEFINITION_LINE_HEADER,
    WENDAO_DEFINITION_PATH_HEADER, WENDAO_DEFINITION_QUERY_HEADER, WENDAO_GRAPH_DIRECTION_HEADER,
    WENDAO_GRAPH_HOPS_HEADER, WENDAO_GRAPH_LIMIT_HEADER, WENDAO_GRAPH_NODE_ID_HEADER,
    WENDAO_SCHEMA_VERSION_HEADER, WENDAO_SEARCH_INTENT_HEADER, WENDAO_SEARCH_LIMIT_HEADER,
    WENDAO_SEARCH_QUERY_HEADER, WENDAO_SEARCH_REPO_HEADER, WENDAO_VFS_PATH_HEADER,
    flight_descriptor_path,
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
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                "query_embedding",
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
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

    let error = validate_rerank_top_k_header(&metadata).expect_err("zero rerank top_k should fail");

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

    let error =
        validate_rerank_top_k_header(&metadata).expect_err("non-numeric rerank top_k should fail");

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
fn validate_definition_request_metadata_accepts_stable_request() {
    let metadata = build_definition_metadata("AlphaService", Some("src/lib.rs"), Some("7"));

    let (query_text, source_path, source_line) = validate_definition_request_metadata(&metadata)
        .expect("stable definition metadata should validate");

    assert_eq!(query_text, "AlphaService");
    assert_eq!(source_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(source_line, Some(7));
}

#[test]
fn validate_definition_request_metadata_rejects_non_numeric_line_hint() {
    let metadata = build_definition_metadata("AlphaService", Some("src/lib.rs"), Some("abc"));

    let error = validate_definition_request_metadata(&metadata)
        .expect_err("non-numeric definition line hint should fail");

    assert_eq!(
        error.message(),
        "invalid definition line header `x-wendao-definition-line`: abc"
    );
}

#[test]
fn validate_autocomplete_request_metadata_accepts_stable_request() {
    let metadata = build_autocomplete_metadata("Alpha", "5");

    let (prefix, limit) = validate_autocomplete_request_metadata(&metadata)
        .expect("stable autocomplete metadata should validate");

    assert_eq!(prefix, "Alpha");
    assert_eq!(limit, 5);
}

#[test]
fn validate_autocomplete_request_metadata_rejects_zero_limit() {
    let metadata = build_autocomplete_metadata("Alpha", "0");

    let error = validate_autocomplete_request_metadata(&metadata)
        .expect_err("zero autocomplete limit should fail");

    assert_eq!(
        error.message(),
        "autocomplete limit must be greater than zero"
    );
}

#[test]
fn validate_vfs_resolve_request_metadata_accepts_stable_request() {
    let metadata = build_vfs_resolve_metadata("main/docs/index.md");

    let path = validate_vfs_resolve_request_metadata(&metadata)
        .expect("stable VFS resolve metadata should validate");

    assert_eq!(path, "main/docs/index.md");
}

#[test]
fn validate_vfs_resolve_request_metadata_rejects_blank_path() {
    let metadata = build_vfs_resolve_metadata("   ");

    let error = validate_vfs_resolve_request_metadata(&metadata)
        .expect_err("blank VFS resolve path should fail");

    assert_eq!(error.message(), "VFS resolve requires a non-empty path");
}

#[test]
fn validate_graph_neighbors_request_metadata_accepts_stable_request() {
    let metadata = build_graph_neighbors_metadata(
        "kernel/docs/index.md",
        Some("outgoing"),
        Some("3"),
        Some("25"),
    );

    let request = validate_graph_neighbors_request_metadata(&metadata)
        .expect("stable graph-neighbors metadata should validate");

    assert_eq!(
        request,
        (
            "kernel/docs/index.md".to_string(),
            "outgoing".to_string(),
            3,
            25,
        )
    );
}

#[test]
fn validate_graph_neighbors_request_metadata_normalizes_defaults() {
    let metadata =
        build_graph_neighbors_metadata("kernel/docs/index.md", Some("invalid"), None, None);

    let request = validate_graph_neighbors_request_metadata(&metadata)
        .expect("graph-neighbors metadata should normalize defaults");

    assert_eq!(
        request,
        (
            "kernel/docs/index.md".to_string(),
            "both".to_string(),
            2,
            50,
        )
    );
}

#[test]
fn validate_graph_neighbors_request_metadata_rejects_invalid_limit() {
    let metadata = build_graph_neighbors_metadata(
        "kernel/docs/index.md",
        Some("both"),
        Some("2"),
        Some("abc"),
    );

    let error = validate_graph_neighbors_request_metadata(&metadata)
        .expect_err("non-numeric graph-neighbors limit should fail");

    assert_eq!(
        error.message(),
        "invalid graph neighbors limit header `x-wendao-graph-limit`: abc"
    );
}

#[test]
fn search_family_route_matcher_accepts_semantic_business_routes() {
    assert!(is_search_family_route(SEARCH_INTENT_ROUTE));
    assert!(is_search_family_route(SEARCH_KNOWLEDGE_ROUTE));
    assert!(!is_search_family_route(SEARCH_ATTACHMENTS_ROUTE));
    assert!(!is_search_family_route(SEARCH_AST_ROUTE));
    assert!(!is_search_family_route(VFS_RESOLVE_ROUTE));
    assert!(!is_search_family_route(GRAPH_NEIGHBORS_ROUTE));
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
async fn wendao_flight_service_do_get_reuses_cached_search_family_payload_after_get_flight_info() {
    let provider = Arc::new(RecordingSearchProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        Some(provider.clone()),
        None,
        None,
        None,
        None,
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
    populate_schema_and_search_headers(flight_info_request.metadata_mut(), "semantic-route", "5");
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
async fn wendao_flight_service_do_get_reuses_cached_search_family_encoded_frames() {
    let provider = Arc::new(RecordingSearchProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        Some(provider.clone()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        3,
        RerankScoreWeights::default(),
    )
    .expect("service should build with search-family provider");

    let mut first_request = Request::new(Ticket::new(SEARCH_INTENT_ROUTE.to_string()));
    populate_schema_and_search_headers(first_request.metadata_mut(), "semantic-route", "6");
    let first_frames = service
        .do_get(first_request)
        .await
        .expect("first DoGet should resolve through the pluggable provider")
        .into_inner()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|frame| frame.expect("first DoGet frame should stream successfully"))
        .collect::<Vec<FlightData>>();

    let mut second_request = Request::new(Ticket::new(SEARCH_INTENT_ROUTE.to_string()));
    populate_schema_and_search_headers(second_request.metadata_mut(), "semantic-route", "6");
    let second_frames = service
        .do_get(second_request)
        .await
        .expect("second DoGet should reuse the cached encoded frames")
        .into_inner()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|frame| frame.expect("second DoGet frame should stream successfully"))
        .collect::<Vec<FlightData>>();

    assert!(!first_frames.is_empty());
    assert_eq!(first_frames, second_frames);
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_reuses_cached_search_family_payload() {
    let provider = Arc::new(RecordingSearchProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        Some(provider.clone()),
        None,
        None,
        None,
        None,
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

    let mut first_request = Request::new(descriptor.clone());
    populate_schema_and_search_headers(first_request.metadata_mut(), "semantic-route", "5");
    let first_info = service
        .get_flight_info(first_request)
        .await
        .expect("first search-family route request should resolve")
        .into_inner();

    let mut second_request = Request::new(descriptor);
    populate_schema_and_search_headers(second_request.metadata_mut(), "semantic-route", "5");
    let second_info = service
        .get_flight_info(second_request)
        .await
        .expect("second search-family route request should reuse the cached payload")
        .into_inner();

    assert_eq!(provider.call_count(), 1);
    assert_eq!(first_info.total_records, second_info.total_records);
    assert_eq!(first_info.app_metadata, second_info.app_metadata);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_definition_provider() {
    let provider = Arc::new(RecordingDefinitionProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        None,
        None,
        None,
        Some(provider.clone()),
        None,
        None,
        None,
        None,
        None,
        3,
        RerankScoreWeights::default(),
    )
    .expect("service should build with definition provider");
    let descriptor = FlightDescriptor::new_path(
        flight_descriptor_path(SEARCH_DEFINITION_ROUTE).expect("descriptor path should build"),
    );
    let mut request = Request::new(descriptor);
    populate_schema_and_definition_headers(
        request.metadata_mut(),
        "AlphaService",
        Some("src/lib.rs"),
        Some("7"),
    );

    let response = service
        .get_flight_info(request)
        .await
        .expect("definition route should resolve through the pluggable provider");
    let flight_info = response.into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.as_ref())
        .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
        .expect("definition route should emit one ticket");
    let app_metadata: serde_json::Value =
        serde_json::from_slice(&flight_info.app_metadata).expect("app_metadata should decode");

    assert_eq!(ticket, SEARCH_DEFINITION_ROUTE);
    assert_eq!(app_metadata["query"], "AlphaService");
    assert_eq!(app_metadata["candidateCount"], 1);
    assert_eq!(provider.call_count(), 1);
    assert_eq!(
        provider.recorded_request(),
        Some((
            "AlphaService".to_string(),
            Some("src/lib.rs".to_string()),
            Some(7),
        ))
    );
}

#[tokio::test]
async fn wendao_flight_service_do_get_reuses_definition_provider_batch() {
    let provider = Arc::new(RecordingDefinitionProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        None,
        None,
        None,
        Some(provider.clone()),
        None,
        None,
        None,
        None,
        None,
        3,
        RerankScoreWeights::default(),
    )
    .expect("service should build with definition provider");
    let mut request = Request::new(Ticket::new(SEARCH_DEFINITION_ROUTE.to_string()));
    populate_schema_and_definition_headers(
        request.metadata_mut(),
        "AlphaService",
        Some("src/lib.rs"),
        Some("7"),
    );

    let response = service
        .do_get(request)
        .await
        .expect("definition route should stream through the pluggable provider");
    let frames = response.into_inner().collect::<Vec<_>>().await;

    assert!(!frames.is_empty());
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_autocomplete_provider() {
    let provider = Arc::new(RecordingAutocompleteProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        None,
        None,
        None,
        None,
        Some(provider.clone()),
        None,
        None,
        None,
        None,
        3,
        RerankScoreWeights::default(),
    )
    .expect("service should build with autocomplete provider");
    let descriptor = FlightDescriptor::new_path(
        flight_descriptor_path(SEARCH_AUTOCOMPLETE_ROUTE).expect("descriptor path should build"),
    );
    let mut request = Request::new(descriptor);
    populate_schema_and_autocomplete_headers(request.metadata_mut(), "Alpha", "5");

    let response = service
        .get_flight_info(request)
        .await
        .expect("autocomplete route should resolve through the pluggable provider");
    let flight_info = response.into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.as_ref())
        .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
        .expect("autocomplete route should emit one ticket");
    let app_metadata: serde_json::Value =
        serde_json::from_slice(&flight_info.app_metadata).expect("app_metadata should decode");

    assert_eq!(ticket, SEARCH_AUTOCOMPLETE_ROUTE);
    assert_eq!(app_metadata["prefix"], "Alpha");
    assert_eq!(provider.call_count(), 1);
    assert_eq!(provider.recorded_request(), Some(("Alpha".to_string(), 5)));
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_vfs_resolve_provider() {
    let provider = Arc::new(RecordingVfsResolveProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(provider.clone()),
        None,
        3,
        RerankScoreWeights::default(),
    )
    .expect("service should build with VFS resolve provider");
    let descriptor = FlightDescriptor::new_path(
        flight_descriptor_path(VFS_RESOLVE_ROUTE).expect("descriptor path should build"),
    );
    let mut request = Request::new(descriptor);
    populate_schema_and_vfs_resolve_headers(request.metadata_mut(), "main/docs/index.md");

    let response = service
        .get_flight_info(request)
        .await
        .expect("VFS resolve route should resolve through the pluggable provider");
    let flight_info = response.into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.as_ref())
        .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
        .expect("VFS resolve route should emit one ticket");
    let app_metadata: serde_json::Value =
        serde_json::from_slice(&flight_info.app_metadata).expect("app_metadata should decode");

    assert_eq!(ticket, VFS_RESOLVE_ROUTE);
    assert_eq!(app_metadata["path"], "main/docs/index.md");
    assert_eq!(provider.call_count(), 1);
    assert_eq!(
        provider.recorded_request(),
        Some("main/docs/index.md".to_string())
    );
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_graph_neighbors_provider() {
    let provider = Arc::new(RecordingGraphNeighborsProvider::default());
    let service = WendaoFlightService::new_with_route_providers(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(provider.clone()),
        3,
        RerankScoreWeights::default(),
    )
    .expect("service should build with graph-neighbors provider");
    let descriptor = FlightDescriptor::new_path(
        flight_descriptor_path(GRAPH_NEIGHBORS_ROUTE).expect("descriptor path should build"),
    );
    let mut request = Request::new(descriptor);
    populate_schema_and_graph_neighbors_headers(
        request.metadata_mut(),
        "kernel/docs/index.md",
        Some("incoming"),
        Some("3"),
        Some("25"),
    );

    let response = service
        .get_flight_info(request)
        .await
        .expect("graph-neighbors route should resolve through the pluggable provider");
    let flight_info = response.into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.as_ref())
        .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
        .expect("graph-neighbors route should emit one ticket");

    assert_eq!(ticket, GRAPH_NEIGHBORS_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some((
            "kernel/docs/index.md".to_string(),
            "incoming".to_string(),
            3,
            25,
        ))
    );
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
        None,
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
        None,
        None,
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
        None,
        None,
        Some(provider.clone()),
        None,
        None,
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
    let app_metadata: serde_json::Value =
        serde_json::from_slice(&flight_info.app_metadata).expect("app_metadata should decode");
    assert_eq!(app_metadata["path"], "docs/analysis.md");
    assert_eq!(app_metadata["documentHash"], "fp:markdown");
    assert_eq!(app_metadata["nodeCount"], 1);
    assert_eq!(app_metadata["edgeCount"], 0);
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
        None,
        None,
        Some(provider.clone()),
        None,
        None,
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
    let app_metadata: serde_json::Value =
        serde_json::from_slice(&flight_info.app_metadata).expect("app_metadata should decode");
    assert_eq!(app_metadata["repoId"], "demo");
    assert_eq!(app_metadata["path"], "src/lib.jl");
    assert_eq!(app_metadata["language"], "julia");
    assert_eq!(app_metadata["nodeCount"], 1);
    assert_eq!(app_metadata["edgeCount"], 0);
    assert_eq!(app_metadata["focusNodeId"], "line:7");
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
    fn recorded_request(&self) -> Option<(String, String, usize, Option<String>, Option<String>)> {
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
struct RecordingDefinitionProvider {
    request: std::sync::Mutex<Option<(String, Option<String>, Option<usize>)>>,
    call_count: std::sync::Mutex<usize>,
}

impl RecordingDefinitionProvider {
    fn recorded_request(&self) -> Option<(String, Option<String>, Option<usize>)> {
        self.request
            .lock()
            .expect("definition provider record should lock")
            .clone()
    }

    fn call_count(&self) -> usize {
        *self
            .call_count
            .lock()
            .expect("definition provider call count should lock")
    }
}

#[async_trait]
impl DefinitionFlightRouteProvider for RecordingDefinitionProvider {
    async fn definition_batch(
        &self,
        query_text: &str,
        source_path: Option<&str>,
        source_line: Option<usize>,
    ) -> Result<DefinitionFlightRouteResponse, tonic::Status> {
        *self
            .request
            .lock()
            .expect("definition provider record should lock") = Some((
            query_text.to_string(),
            source_path.map(ToString::to_string),
            source_line,
        ));
        *self
            .call_count
            .lock()
            .expect("definition provider call count should lock") += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("name", LanceDataType::Utf8, false),
                LanceField::new("path", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![query_text.to_string()])),
                Arc::new(StringArray::from(vec![
                    source_path.unwrap_or("src/lib.rs").to_string(),
                ])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(DefinitionFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "query": query_text,
                "sourcePath": source_path,
                "sourceLine": source_line,
                "candidateCount": 1,
                "selectedScope": "definition",
            })
            .to_string()
            .into_bytes(),
        ))
    }
}

#[derive(Debug, Default)]
struct RecordingAutocompleteProvider {
    request: std::sync::Mutex<Option<(String, usize)>>,
    call_count: std::sync::Mutex<usize>,
}

impl RecordingAutocompleteProvider {
    fn recorded_request(&self) -> Option<(String, usize)> {
        self.request
            .lock()
            .expect("autocomplete provider record should lock")
            .clone()
    }

    fn call_count(&self) -> usize {
        *self
            .call_count
            .lock()
            .expect("autocomplete provider call count should lock")
    }
}

#[async_trait]
impl AutocompleteFlightRouteProvider for RecordingAutocompleteProvider {
    async fn autocomplete_batch(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<AutocompleteFlightRouteResponse, tonic::Status> {
        *self
            .request
            .lock()
            .expect("autocomplete provider record should lock") = Some((prefix.to_string(), limit));
        *self
            .call_count
            .lock()
            .expect("autocomplete provider call count should lock") += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("text", LanceDataType::Utf8, false),
                LanceField::new("suggestionType", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!("{prefix}_suggestion")])),
                Arc::new(StringArray::from(vec!["symbol"])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(
            AutocompleteFlightRouteResponse::new(batch).with_app_metadata(
                serde_json::json!({
                    "prefix": prefix,
                })
                .to_string()
                .into_bytes(),
            ),
        )
    }
}

#[derive(Debug, Default)]
struct RecordingVfsResolveProvider {
    request: std::sync::Mutex<Option<String>>,
    call_count: std::sync::Mutex<usize>,
}

impl RecordingVfsResolveProvider {
    fn recorded_request(&self) -> Option<String> {
        self.request
            .lock()
            .expect("VFS resolve provider record should lock")
            .clone()
    }

    fn call_count(&self) -> usize {
        *self
            .call_count
            .lock()
            .expect("VFS resolve provider call count should lock")
    }
}

#[async_trait]
impl VfsResolveFlightRouteProvider for RecordingVfsResolveProvider {
    async fn resolve_vfs_navigation_batch(
        &self,
        path: &str,
    ) -> Result<VfsResolveFlightRouteResponse, tonic::Status> {
        *self
            .request
            .lock()
            .expect("VFS resolve provider record should lock") = Some(path.to_string());
        *self
            .call_count
            .lock()
            .expect("VFS resolve provider call count should lock") += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("path", LanceDataType::Utf8, false),
                LanceField::new("category", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![path.to_string()])),
                Arc::new(StringArray::from(vec!["file".to_string()])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(VfsResolveFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "path": path,
                "navigationTarget": {
                    "path": path,
                    "category": "file",
                },
            })
            .to_string()
            .into_bytes(),
        ))
    }
}

#[derive(Debug, Default)]
struct RecordingGraphNeighborsProvider {
    request: std::sync::Mutex<Option<(String, String, usize, usize)>>,
    call_count: std::sync::Mutex<usize>,
}

impl RecordingGraphNeighborsProvider {
    fn recorded_request(&self) -> Option<(String, String, usize, usize)> {
        self.request
            .lock()
            .expect("graph-neighbors provider record should lock")
            .clone()
    }

    fn call_count(&self) -> usize {
        *self
            .call_count
            .lock()
            .expect("graph-neighbors provider call count should lock")
    }
}

#[async_trait]
impl GraphNeighborsFlightRouteProvider for RecordingGraphNeighborsProvider {
    async fn graph_neighbors_batch(
        &self,
        node_id: &str,
        direction: &str,
        hops: usize,
        limit: usize,
    ) -> Result<GraphNeighborsFlightRouteResponse, tonic::Status> {
        *self
            .request
            .lock()
            .expect("graph-neighbors provider record should lock") =
            Some((node_id.to_string(), direction.to_string(), hops, limit));
        *self
            .call_count
            .lock()
            .expect("graph-neighbors provider call count should lock") += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("rowType", LanceDataType::Utf8, false),
                LanceField::new("nodeId", LanceDataType::Utf8, true),
                LanceField::new("nodeLabel", LanceDataType::Utf8, true),
                LanceField::new("nodePath", LanceDataType::Utf8, true),
                LanceField::new("nodeType", LanceDataType::Utf8, true),
                LanceField::new("nodeIsCenter", LanceDataType::Boolean, true),
                LanceField::new("nodeDistance", LanceDataType::Int32, true),
                LanceField::new("navigationPath", LanceDataType::Utf8, true),
                LanceField::new("navigationCategory", LanceDataType::Utf8, true),
                LanceField::new("navigationProjectName", LanceDataType::Utf8, true),
                LanceField::new("navigationRootLabel", LanceDataType::Utf8, true),
                LanceField::new("navigationLine", LanceDataType::Int32, true),
                LanceField::new("navigationLineEnd", LanceDataType::Int32, true),
                LanceField::new("navigationColumn", LanceDataType::Int32, true),
                LanceField::new("linkSource", LanceDataType::Utf8, true),
                LanceField::new("linkTarget", LanceDataType::Utf8, true),
                LanceField::new("linkDirection", LanceDataType::Utf8, true),
                LanceField::new("linkDistance", LanceDataType::Int32, true),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["node", "link"])),
                Arc::new(StringArray::from(vec![Some(node_id.to_string()), None])),
                Arc::new(StringArray::from(vec![Some("Index".to_string()), None])),
                Arc::new(StringArray::from(vec![Some(node_id.to_string()), None])),
                Arc::new(StringArray::from(vec![Some("doc".to_string()), None])),
                Arc::new(LanceBooleanArray::from(vec![Some(true), None])),
                Arc::new(LanceInt32Array::from(vec![Some(0), None])),
                Arc::new(StringArray::from(vec![Some(node_id.to_string()), None])),
                Arc::new(StringArray::from(vec![Some("doc".to_string()), None])),
                Arc::new(StringArray::from(vec![Some("kernel".to_string()), None])),
                Arc::new(StringArray::from(vec![Some("project".to_string()), None])),
                Arc::new(LanceInt32Array::from(vec![Some(7), None])),
                Arc::new(LanceInt32Array::from(vec![Some(9), None])),
                Arc::new(LanceInt32Array::from(vec![Some(3), None])),
                Arc::new(StringArray::from(vec![None, Some(node_id.to_string())])),
                Arc::new(StringArray::from(vec![
                    None,
                    Some(format!("{node_id}::neighbor")),
                ])),
                Arc::new(StringArray::from(vec![None, Some(direction.to_string())])),
                Arc::new(LanceInt32Array::from(vec![
                    None,
                    Some(i32::try_from(hops.min(limit)).unwrap_or(i32::MAX)),
                ])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(GraphNeighborsFlightRouteResponse::new(batch))
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
    async fn markdown_analysis_batch(
        &self,
        path: &str,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        *self
            .request
            .lock()
            .expect("markdown analysis provider record should lock") = Some(path.to_string());
        let batch = LanceRecordBatch::try_new(
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
        .map_err(|error| error.to_string())?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::to_vec(&serde_json::json!({
                "path": path,
                "documentHash": "fp:markdown",
                "nodeCount": 1,
                "edgeCount": 0,
                "nodes": [],
                "edges": [],
                "projections": [],
                "diagnostics": [],
            }))
            .map_err(|error| error.to_string())?,
        ))
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
    ) -> Result<AnalysisFlightRouteResponse, String> {
        *self
            .request
            .lock()
            .expect("code-AST analysis provider record should lock") =
            Some((path.to_string(), repo_id.to_string(), line_hint));
        let batch = LanceRecordBatch::try_new(
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
        .map_err(|error| error.to_string())?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::to_vec(&serde_json::json!({
                "repoId": repo_id,
                "path": path,
                "language": "julia",
                "nodeCount": 1,
                "edgeCount": 0,
                "nodes": [],
                "edges": [],
                "projections": [],
                "focusNodeId": line_hint.map(|line| format!("line:{line}")),
                "diagnostics": [],
            }))
            .map_err(|error| error.to_string())?,
        ))
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

fn build_definition_metadata(
    query_text: &str,
    source_path: Option<&str>,
    source_line: Option<&str>,
) -> MetadataMap {
    let mut metadata = MetadataMap::new();
    populate_schema_and_definition_headers(&mut metadata, query_text, source_path, source_line);
    metadata
}

fn build_autocomplete_metadata(prefix: &str, limit: &str) -> MetadataMap {
    let mut metadata = MetadataMap::new();
    populate_schema_and_autocomplete_headers(&mut metadata, prefix, limit);
    metadata
}

fn build_vfs_resolve_metadata(path: &str) -> MetadataMap {
    let mut metadata = MetadataMap::new();
    populate_schema_and_vfs_resolve_headers(&mut metadata, path);
    metadata
}

fn build_graph_neighbors_metadata(
    node_id: &str,
    direction: Option<&str>,
    hops: Option<&str>,
    limit: Option<&str>,
) -> MetadataMap {
    let mut metadata = MetadataMap::new();
    populate_schema_and_graph_neighbors_headers(&mut metadata, node_id, direction, hops, limit);
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

fn populate_schema_and_search_headers(metadata: &mut MetadataMap, query_text: &str, limit: &str) {
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
            MetadataValue::try_from(intent).expect("search-family intent metadata should parse"),
        );
    }
    if let Some(repo_hint) = repo_hint {
        metadata.insert(
            WENDAO_SEARCH_REPO_HEADER,
            MetadataValue::try_from(repo_hint).expect("search-family repo metadata should parse"),
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

fn populate_schema_and_definition_headers(
    metadata: &mut MetadataMap,
    query_text: &str,
    source_path: Option<&str>,
    source_line: Option<&str>,
) {
    metadata.insert(
        WENDAO_SCHEMA_VERSION_HEADER,
        MetadataValue::try_from("v2").expect("schema version metadata should parse"),
    );
    metadata.insert(
        WENDAO_DEFINITION_QUERY_HEADER,
        MetadataValue::try_from(query_text).expect("definition query metadata should parse"),
    );
    if let Some(source_path) = source_path {
        metadata.insert(
            WENDAO_DEFINITION_PATH_HEADER,
            MetadataValue::try_from(source_path).expect("definition path metadata should parse"),
        );
    }
    if let Some(source_line) = source_line {
        metadata.insert(
            WENDAO_DEFINITION_LINE_HEADER,
            MetadataValue::try_from(source_line).expect("definition line metadata should parse"),
        );
    }
}

fn populate_schema_and_autocomplete_headers(metadata: &mut MetadataMap, prefix: &str, limit: &str) {
    metadata.insert(
        WENDAO_SCHEMA_VERSION_HEADER,
        MetadataValue::try_from("v2").expect("schema version metadata should parse"),
    );
    metadata.insert(
        WENDAO_AUTOCOMPLETE_PREFIX_HEADER,
        MetadataValue::try_from(prefix).expect("autocomplete prefix metadata should parse"),
    );
    metadata.insert(
        WENDAO_AUTOCOMPLETE_LIMIT_HEADER,
        MetadataValue::try_from(limit).expect("autocomplete limit metadata should parse"),
    );
}

fn populate_schema_and_vfs_resolve_headers(metadata: &mut MetadataMap, path: &str) {
    metadata.insert(
        WENDAO_SCHEMA_VERSION_HEADER,
        MetadataValue::try_from("v2").expect("schema version metadata should parse"),
    );
    metadata.insert(
        WENDAO_VFS_PATH_HEADER,
        MetadataValue::try_from(path).expect("VFS resolve path metadata should parse"),
    );
}

fn populate_schema_and_graph_neighbors_headers(
    metadata: &mut MetadataMap,
    node_id: &str,
    direction: Option<&str>,
    hops: Option<&str>,
    limit: Option<&str>,
) {
    metadata.insert(
        WENDAO_SCHEMA_VERSION_HEADER,
        MetadataValue::try_from("v2").expect("schema version metadata should parse"),
    );
    metadata.insert(
        WENDAO_GRAPH_NODE_ID_HEADER,
        MetadataValue::try_from(node_id).expect("graph-neighbors node id metadata should parse"),
    );
    if let Some(direction) = direction {
        metadata.insert(
            WENDAO_GRAPH_DIRECTION_HEADER,
            MetadataValue::try_from(direction)
                .expect("graph-neighbors direction metadata should parse"),
        );
    }
    if let Some(hops) = hops {
        metadata.insert(
            WENDAO_GRAPH_HOPS_HEADER,
            MetadataValue::try_from(hops).expect("graph-neighbors hops metadata should parse"),
        );
    }
    if let Some(limit) = limit {
        metadata.insert(
            WENDAO_GRAPH_LIMIT_HEADER,
            MetadataValue::try_from(limit).expect("graph-neighbors limit metadata should parse"),
        );
    }
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
