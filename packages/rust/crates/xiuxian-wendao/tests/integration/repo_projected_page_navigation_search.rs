//! Integration tests for deterministic projected page navigation search.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectionPageKind, RepoProjectedPageNavigationSearchQuery,
    build_repo_projected_page_navigation_search,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_navigation_search_expands_reference_hits_into_navigation_bundles() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");
    let result = build_repo_projected_page_navigation_search(
        &RepoProjectedPageNavigationSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            family_kind: None,
            limit: 3,
            related_limit: 3,
            family_limit: 2,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot(
        "repo_projected_page_navigation_search_result",
        json!(result),
    );
    Ok(())
}
