//! Integration tests for deterministic projected page-index tree lookup.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreesQuery,
    build_repo_projected_page_index_tree, build_repo_projected_page_index_trees,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_index_tree_lookup_resolves_one_stable_tree() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let trees = build_repo_projected_page_index_trees(
        &RepoProjectedPageIndexTreesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    )?;

    let page_id = trees
        .trees
        .iter()
        .find(|tree| tree.title == "solve")
        .map(|tree| tree.page_id.clone())
        .expect("expected a projected page-index tree titled `solve`");

    let result = build_repo_projected_page_index_tree(
        &RepoProjectedPageIndexTreeQuery {
            repo_id: "projection-sample".to_string(),
            page_id,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot("repo_projected_page_index_tree_result", json!(result));
    Ok(())
}
