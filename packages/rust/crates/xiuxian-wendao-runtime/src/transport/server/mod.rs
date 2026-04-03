mod flight;
mod request_metadata;
mod types;

pub use flight::WendaoFlightService;
pub use types::{
    AnalysisFlightRouteResponse, AstSearchFlightRouteProvider, AttachmentSearchFlightRouteProvider,
    AutocompleteFlightRouteProvider, AutocompleteFlightRouteResponse,
    CodeAstAnalysisFlightRouteProvider, DefinitionFlightRouteProvider,
    DefinitionFlightRouteResponse, GraphNeighborsFlightRouteProvider,
    GraphNeighborsFlightRouteResponse, MarkdownAnalysisFlightRouteProvider,
    RepoSearchFlightRequest, RepoSearchFlightRouteProvider, RerankFlightRouteHandler,
    SearchFlightRouteProvider, SearchFlightRouteResponse, SqlFlightRouteProvider,
    SqlFlightRouteResponse, VfsResolveFlightRouteProvider, VfsResolveFlightRouteResponse,
    WendaoFlightRouteProviders,
};

#[cfg(test)]
pub(crate) use request_metadata::{
    is_search_family_route, validate_attachment_search_request_metadata,
    validate_autocomplete_request_metadata, validate_code_ast_analysis_request_metadata,
    validate_definition_request_metadata, validate_graph_neighbors_request_metadata,
    validate_markdown_analysis_request_metadata, validate_rerank_top_k_header,
    validate_search_request_metadata, validate_sql_request_metadata,
    validate_vfs_resolve_request_metadata,
};
