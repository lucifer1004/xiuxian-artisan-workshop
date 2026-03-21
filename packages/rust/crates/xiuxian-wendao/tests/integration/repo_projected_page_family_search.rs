//! Integration tests for deterministic projected page-family search.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    ProjectionPageKind, RepoProjectedPageFamilySearchQuery,
    repo_projected_page_family_search_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_family_search_matches_reference_family_clusters() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_julia_repo(temp.path(), "ProjectionPkg", true)?;
    let config_path = write_repo_config(temp.path(), &repo_dir, "projection-sample")?;

    let result = repo_projected_page_family_search_from_config(
        &RepoProjectedPageFamilySearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            limit: 5,
            per_kind_limit: 2,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot("repo_projected_page_family_search_result", json!(result));
    Ok(())
}
