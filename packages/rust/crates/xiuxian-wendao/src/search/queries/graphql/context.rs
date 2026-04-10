use crate::search::queries::SearchQueryService;

/// Execution context shared by the GraphQL query adapter slice.
#[derive(Clone, Default)]
pub(crate) struct GraphqlExecutionContext {
    query_service: Option<SearchQueryService>,
}

impl GraphqlExecutionContext {
    /// Build an empty context and attach the required surfaces incrementally.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Attach the shared query service used by GraphQL table-query fields.
    #[must_use]
    pub(crate) fn with_query_service(mut self, query_service: SearchQueryService) -> Self {
        self.query_service = Some(query_service);
        self
    }
    pub(crate) fn query_service(&self) -> Option<&SearchQueryService> {
        self.query_service.as_ref()
    }
}
