//! Integration tests for deterministic projected-page search.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectionPageKind, RepoProjectedPageSearchQuery, build_repo_projected_page_search,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_search_matches_reference_pages() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");
    let result = build_repo_projected_page_search(
        &RepoProjectedPageSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            limit: 10,
        },
        &analysis,
    );

    assert_repo_json_snapshot("repo_projected_page_search_result", json!(result));
    Ok(())
}
