use tempfile::TempDir;

use crate::search::queries::SearchQueryService;
use crate::search::queries::tests::fixtures as shared_fixtures;
use crate::search_plane::SearchPlaneService;

pub(super) fn fixture_service(temp_dir: &TempDir) -> SearchPlaneService {
    shared_fixtures::fixture_service(temp_dir, "xiuxian:test:studio_rest")
}

pub(super) fn query_service(service: SearchPlaneService) -> SearchQueryService {
    SearchQueryService::new(service)
}

pub(super) use shared_fixtures::{publish_reference_hits, sample_hit};
