//! Integration tests for deterministic projected page navigation search.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    ProjectionPageKind, RepoProjectedPageNavigationSearchQuery,
    repo_projected_page_navigation_search_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_navigation_search_expands_reference_hits_into_navigation_bundles() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_julia_repo(temp.path(), "ProjectionPkg", true)?;
    let config_path = write_repo_config(temp.path(), &repo_dir, "projection-sample")?;

    let result = repo_projected_page_navigation_search_from_config(
        &RepoProjectedPageNavigationSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            family_kind: None,
            limit: 3,
            related_limit: 3,
            family_limit: 2,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot(
        "repo_projected_page_navigation_search_result",
        json!(result),
    );
    Ok(())
}
