//! Integration tests for deterministic projected-page lookup.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    RepoProjectedPageQuery, RepoProjectedPagesQuery, repo_projected_page_from_config,
    repo_projected_pages_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_lookup_resolves_one_stable_page() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_julia_repo(temp.path(), "ProjectionPkg", true)?;
    let config_path = write_repo_config(temp.path(), &repo_dir, "projection-sample")?;

    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "projection-sample".to_string(),
        },
        Some(&config_path),
        temp.path(),
    )?;

    let page_id = pages
        .pages
        .iter()
        .find(|page| page.title == "solve")
        .map(|page| page.page_id.clone())
        .expect("expected a projected page titled `solve`");

    let result = repo_projected_page_from_config(
        &RepoProjectedPageQuery {
            repo_id: "projection-sample".to_string(),
            page_id,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot("repo_projected_page_result", json!(result));
    Ok(())
}
