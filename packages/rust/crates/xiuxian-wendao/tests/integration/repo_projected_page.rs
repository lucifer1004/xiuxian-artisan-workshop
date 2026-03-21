//! Integration tests for deterministic projected-page lookup.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    RepoProjectedPageQuery, RepoProjectedPagesQuery, build_repo_projected_page,
    build_repo_projected_pages,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_lookup_resolves_one_stable_page() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let pages = build_repo_projected_pages(
        &RepoProjectedPagesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );

    let page_id = pages
        .pages
        .iter()
        .find(|page| page.title == "solve")
        .map(|page| page.page_id.clone())
        .expect("expected a projected page titled `solve`");

    let result = build_repo_projected_page(
        &RepoProjectedPageQuery {
            repo_id: "projection-sample".to_string(),
            page_id,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot("repo_projected_page_result", json!(result));
    Ok(())
}
