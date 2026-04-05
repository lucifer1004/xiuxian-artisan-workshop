use tempfile::TempDir;

use crate::search::queries::SearchQueryService;
use crate::search::queries::graphql::context::GraphqlExecutionContext;
use crate::search::queries::tests::fixtures as shared_fixtures;
use crate::search_plane::SearchPlaneService;

pub(super) fn fixture_service(temp_dir: &TempDir) -> SearchPlaneService {
    shared_fixtures::fixture_service(temp_dir, "xiuxian:test:studio_graphql")
}

pub(super) fn graphql_context(search_plane: SearchPlaneService) -> GraphqlExecutionContext {
    GraphqlExecutionContext::new().with_query_service(SearchQueryService::new(search_plane))
}

pub(super) use shared_fixtures::{publish_reference_hits, sample_hit};
