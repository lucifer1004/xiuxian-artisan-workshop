//! Integration tests for deterministic mixed projected retrieval context.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    ProjectedPageIndexNode, RepoProjectedPageIndexTreesQuery, RepoProjectedPagesQuery,
    RepoProjectedRetrievalContextQuery, repo_projected_page_index_trees_from_config,
    repo_projected_pages_from_config, repo_projected_retrieval_context_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_retrieval_context_lookup_resolves_page_context() -> TestResult {
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
    let page = pages
        .pages
        .iter()
        .find(|page| page.title == "solve")
        .expect("expected a projected page titled `solve`");

    let result = repo_projected_retrieval_context_from_config(
        &RepoProjectedRetrievalContextQuery {
            repo_id: "projection-sample".to_string(),
            page_id: page.page_id.clone(),
            node_id: None,
            related_limit: 3,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot(
        "repo_projected_retrieval_context_page_result",
        json!(result),
    );
    Ok(())
}

#[test]
fn projected_retrieval_context_lookup_resolves_node_context() -> TestResult {
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

    let result = repo_projected_retrieval_context_from_config(
        &RepoProjectedRetrievalContextQuery {
            repo_id: "projection-sample".to_string(),
            page_id: tree.page_id.clone(),
            node_id: Some(node_id),
            related_limit: 3,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot(
        "repo_projected_retrieval_context_node_result",
        json!(result),
    );
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
