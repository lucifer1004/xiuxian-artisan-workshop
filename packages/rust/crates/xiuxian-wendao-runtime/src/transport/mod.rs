#[cfg(feature = "julia")]
mod client;
#[cfg(feature = "julia")]
mod flight;
#[cfg(feature = "julia")]
mod negotiation;
mod query_contract;
#[cfg(feature = "julia")]
mod server;

#[cfg(feature = "julia")]
pub use client::build_arrow_transport_client_from_binding;
#[cfg(feature = "julia")]
pub use negotiation::{
    CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, NegotiatedArrowTransportClient,
    NegotiatedTransportSelection, negotiate_arrow_transport_client_from_bindings,
};
pub use query_contract::{
    REPO_SEARCH_BEST_SECTION_COLUMN, REPO_SEARCH_DEFAULT_LIMIT, REPO_SEARCH_DOC_ID_COLUMN,
    REPO_SEARCH_LANGUAGE_COLUMN, REPO_SEARCH_PATH_COLUMN, REPO_SEARCH_ROUTE,
    REPO_SEARCH_SCORE_COLUMN, REPO_SEARCH_TITLE_COLUMN,
    RERANK_EXCHANGE_ROUTE, RERANK_REQUEST_DOC_ID_COLUMN, RERANK_REQUEST_EMBEDDING_COLUMN,
    RERANK_REQUEST_QUERY_EMBEDDING_COLUMN, RERANK_REQUEST_VECTOR_SCORE_COLUMN,
    RERANK_RESPONSE_DOC_ID_COLUMN, RERANK_RESPONSE_FINAL_SCORE_COLUMN, RERANK_RESPONSE_RANK_COLUMN,
    WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER, WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER, WENDAO_REPO_SEARCH_LIMIT_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER, WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER, WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
    WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_SCHEMA_VERSION_HEADER, flight_descriptor_path, normalize_flight_route,
    validate_repo_search_request,
};
#[cfg(feature = "julia")]
pub use query_contract::{
    score_rerank_request_batch, validate_rerank_request_batch, validate_rerank_request_schema,
    validate_rerank_response_batch, validate_rerank_response_schema,
};
#[cfg(feature = "julia")]
pub use server::{RepoSearchFlightRouteProvider, RerankFlightRouteHandler, WendaoFlightService};
