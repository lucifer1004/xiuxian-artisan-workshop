//! Integration tests for deterministic projected page-index node lookup.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    ProjectedPageIndexNode, RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexTreesQuery,
    repo_projected_page_index_node_from_config, repo_projected_page_index_trees_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_index_node_lookup_resolves_one_stable_node() -> TestResult {
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

    let tree = trees
        .trees
        .iter()
        .find(|tree| tree.title == "solve")
        .expect("expected a projected page-index tree titled `solve`");
    let node_id = find_node_id(tree.roots.as_slice(), "Anchors")
        .expect("expected a projected page-index node titled `Anchors`");

    let result = repo_projected_page_index_node_from_config(
        &RepoProjectedPageIndexNodeQuery {
            repo_id: "projection-sample".to_string(),
            page_id: tree.page_id.clone(),
            node_id,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot("repo_projected_page_index_node_result", json!(result));
    Ok(())
}

fn find_node_id(nodes: &[ProjectedPageIndexNode], title: &str) -> Option<String> {
    for node in nodes {
        if node.title == title {
            return Some(node.node_id.clone());
        }
        if let Some(node_id) = find_node_id(node.children.as_slice(), title) {
            return Some(node_id);
        }
    }
    None
}
