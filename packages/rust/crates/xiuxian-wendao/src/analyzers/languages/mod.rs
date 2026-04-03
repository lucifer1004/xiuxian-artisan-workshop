//! Language-specific Repo Intelligence plugins bundled into the Wendao runtime.
//!
//! The Julia plugin now enters the host through a normal crate dependency.
//! Modelica now follows the same package-dependency path, which turns the
//! second plugin onboarding proof into a normal Cargo integration rather than
//! a sibling-source inclusion seam.

#[cfg(feature = "julia")]
pub use xiuxian_wendao_julia::{
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN, GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
    GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN,
    GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
    GraphStructuralCandidateSubgraph, GraphStructuralFilterConstraint,
    GraphStructuralFilterRequestRow, GraphStructuralFilterScoreRow,
    GraphStructuralKeywordOverlapCandidateInputs, GraphStructuralKeywordOverlapPairInputs,
    GraphStructuralKeywordOverlapPairRequestInputs, GraphStructuralKeywordOverlapPairRerankInputs,
    GraphStructuralKeywordOverlapQueryInputs, GraphStructuralKeywordOverlapRawCandidateInputs,
    GraphStructuralNodeMetadataInputs, GraphStructuralPairCandidateInputs,
    GraphStructuralQueryAnchor, GraphStructuralQueryContext, GraphStructuralRerankRequestRow,
    GraphStructuralRerankScoreRow, GraphStructuralRerankSignals,
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, JuliaRepoIntelligencePlugin,
    build_graph_structural_filter_request_batch, build_graph_structural_filter_request_row,
    build_graph_structural_keyword_overlap_candidate_inputs,
    build_graph_structural_keyword_overlap_pair_candidate_inputs,
    build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw,
    build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs,
    build_graph_structural_keyword_overlap_pair_request_input,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates,
    build_graph_structural_keyword_overlap_pair_rerank_request_row,
    build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata,
    build_graph_structural_keyword_overlap_query_inputs,
    build_graph_structural_keyword_overlap_raw_candidate_inputs,
    build_graph_structural_keyword_tag_pair_rerank_request_row,
    build_graph_structural_keyword_tag_query_context,
    build_graph_structural_keyword_tag_rerank_signals,
    build_graph_structural_pair_candidate_inputs, build_graph_structural_pair_candidate_subgraph,
    build_graph_structural_pair_filter_request_row, build_graph_structural_pair_rerank_request_row,
    build_graph_structural_rerank_request_batch, build_graph_structural_rerank_request_row,
    build_julia_flight_transport_client,
    fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository,
    fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates,
    graph_structural_pair_candidate_id, graph_structural_shared_tag_anchors,
    process_graph_structural_flight_batches_for_repository, process_julia_flight_batches,
    process_julia_flight_batches_for_repository, register_into as register_julia_plugin,
    validate_graph_structural_filter_request_batch,
    validate_graph_structural_filter_response_batch,
    validate_graph_structural_rerank_request_batch,
    validate_graph_structural_rerank_response_batch, validate_julia_arrow_response_batches,
};

#[cfg(feature = "modelica")]
pub use xiuxian_wendao_modelica::{
    ModelicaRepoIntelligencePlugin, register_into as register_modelica_plugin,
};
