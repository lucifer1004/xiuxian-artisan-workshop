mod discovery;
mod entry;
mod graph_structural;
mod graph_structural_transport;
mod linking;
mod project;
mod sources;
#[cfg(test)]
pub(crate) mod test_support;
mod transport;

pub use entry::JuliaRepoIntelligencePlugin;
pub use entry::register_into;
pub use graph_structural::{
    GRAPH_STRUCTURAL_ACCEPTED_COLUMN, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN,
    GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN, GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN,
    GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN, GRAPH_STRUCTURAL_EXPLANATION_COLUMN,
    GRAPH_STRUCTURAL_FEASIBLE_COLUMN, GRAPH_STRUCTURAL_FILTER_REQUEST_COLUMNS,
    GRAPH_STRUCTURAL_FILTER_RESPONSE_COLUMNS, GRAPH_STRUCTURAL_FILTER_ROUTE,
    GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN,
    GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN, GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
    GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN,
    GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN, GRAPH_STRUCTURAL_RERANK_REQUEST_COLUMNS,
    GRAPH_STRUCTURAL_RERANK_RESPONSE_COLUMNS, GRAPH_STRUCTURAL_RERANK_ROUTE,
    GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN, GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN,
    GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
    GraphStructuralRouteKind, JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION, graph_structural_route_kind,
    is_graph_structural_route, validate_graph_structural_filter_request_batch,
    validate_graph_structural_filter_request_schema,
    validate_graph_structural_filter_response_batch,
    validate_graph_structural_filter_response_schema,
    validate_graph_structural_rerank_request_batch,
    validate_graph_structural_rerank_request_schema,
    validate_graph_structural_rerank_response_batch,
    validate_graph_structural_rerank_response_schema,
};
pub use graph_structural_transport::{
    build_graph_structural_flight_transport_client, process_graph_structural_flight_batches,
    process_graph_structural_flight_batches_for_repository,
    validate_graph_structural_request_batches, validate_graph_structural_response_batches,
};
pub use transport::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, build_julia_flight_transport_client,
    process_julia_flight_batches, process_julia_flight_batches_for_repository,
    validate_julia_arrow_response_batches,
};
