//! Integration tests for deterministic projected page-index tree lookup.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreesQuery,
    repo_projected_page_index_tree_from_config, repo_projected_page_index_trees_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_index_tree_lookup_resolves_one_stable_tree() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_julia_repo(temp.path(), "ProjectionPkg", true)?;
    let config_path = write_repo_config(temp.path(), &repo_dir, "projection-sample")?;

    let trees = repo_projected_page_index_trees_from_config(
        &RepoProjectedPageIndexTreesQuery {
            repo_id: "projection-sample".to_string(),
        },
        Some(&config_path),
        temp.path(),
    )?;

    let page_id = trees
        .trees
        .iter()
        .find(|tree| tree.title == "solve")
        .map(|tree| tree.page_id.clone())
        .expect("expected a projected page-index tree titled `solve`");

    let result = repo_projected_page_index_tree_from_config(
        &RepoProjectedPageIndexTreeQuery {
            repo_id: "projection-sample".to_string(),
            page_id,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot("repo_projected_page_index_tree_result", json!(result));
    Ok(())
}
