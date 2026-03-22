//! Integration tests for deterministic projected page-family search.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectionPageKind, RepoProjectedPageFamilySearchQuery, build_repo_projected_page_family_search,
};

#[test]
fn projected_page_family_search_matches_reference_family_clusters() {
    let analysis = sample_projection_analysis("projection-sample");
    let result = build_repo_projected_page_family_search(
        &RepoProjectedPageFamilySearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            limit: 5,
            per_kind_limit: 2,
        },
        &analysis,
    );

    assert_repo_json_snapshot("repo_projected_page_family_search_result", json!(result));
}
