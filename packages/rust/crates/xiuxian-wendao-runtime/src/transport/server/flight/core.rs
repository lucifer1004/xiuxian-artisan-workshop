use std::sync::Arc;

use crate::transport::RerankScoreWeights;

use super::super::types::{
    AstSearchFlightRouteProvider, AttachmentSearchFlightRouteProvider,
    AutocompleteFlightRouteProvider, CodeAstAnalysisFlightRouteProvider,
    DefinitionFlightRouteProvider, GraphNeighborsFlightRouteProvider,
    MarkdownAnalysisFlightRouteProvider, RepoSearchFlightRouteProvider, RerankFlightRouteHandler,
    SearchFlightRouteProvider, SqlFlightRouteProvider, VfsResolveFlightRouteProvider,
    WendaoFlightRouteProviders,
};
use super::cache::FlightRoutePayloadCache;

/// Runtime-owned minimal Wendao Flight service surface for the stable query and
/// rerank routes.
#[derive(Debug, Clone)]
pub struct WendaoFlightService {
    pub(crate) expected_schema_version: String,
    pub(super) repo_search_provider: Arc<dyn RepoSearchFlightRouteProvider>,
    pub(super) search_provider: Option<Arc<dyn SearchFlightRouteProvider>>,
    pub(super) attachment_search_provider: Option<Arc<dyn AttachmentSearchFlightRouteProvider>>,
    pub(super) ast_search_provider: Option<Arc<dyn AstSearchFlightRouteProvider>>,
    pub(super) definition_provider: Option<Arc<dyn DefinitionFlightRouteProvider>>,
    pub(super) autocomplete_provider: Option<Arc<dyn AutocompleteFlightRouteProvider>>,
    pub(super) vfs_resolve_provider: Option<Arc<dyn VfsResolveFlightRouteProvider>>,
    pub(super) graph_neighbors_provider: Option<Arc<dyn GraphNeighborsFlightRouteProvider>>,
    pub(super) markdown_analysis_provider: Option<Arc<dyn MarkdownAnalysisFlightRouteProvider>>,
    pub(super) code_ast_analysis_provider: Option<Arc<dyn CodeAstAnalysisFlightRouteProvider>>,
    pub(super) sql_provider: Option<Arc<dyn SqlFlightRouteProvider>>,
    pub(super) rerank_handler: RerankFlightRouteHandler,
    pub(super) route_payload_cache: Arc<FlightRoutePayloadCache>,
}

impl WendaoFlightService {
    pub(super) fn build(
        expected_schema_version: String,
        route_providers: WendaoFlightRouteProviders,
        rerank_weights: RerankScoreWeights,
        rerank_dimension: usize,
    ) -> Result<Self, String> {
        let WendaoFlightRouteProviders {
            repo_search: repo_search_provider,
            search: search_provider,
            attachment_search: attachment_search_provider,
            ast_search: ast_search_provider,
            definition: definition_provider,
            autocomplete: autocomplete_provider,
            markdown_analysis: markdown_analysis_provider,
            code_ast_analysis: code_ast_analysis_provider,
            vfs_resolve: vfs_resolve_provider,
            graph_neighbors: graph_neighbors_provider,
            sql: sql_provider,
        } = route_providers;
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
            sql_provider,
            rerank_handler: RerankFlightRouteHandler::new_with_weights(
                rerank_dimension,
                rerank_weights,
            )?,
            route_payload_cache: Arc::new(FlightRoutePayloadCache::default()),
        })
    }
}
