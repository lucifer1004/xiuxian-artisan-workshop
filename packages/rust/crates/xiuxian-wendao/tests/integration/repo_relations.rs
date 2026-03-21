//! Integration tests for Repo Intelligence relation graph output.

#[path = "../support/repo_intelligence.rs"]
mod repo_test_support;

use repo_test_support::{assert_repo_json_snapshot, create_sample_julia_repo, write_repo_config};
use serde_json::json;
use xiuxian_wendao::analyzers::analyze_repository_from_config;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn analysis_emits_structural_and_semantic_relations() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_julia_repo(temp.path(), "RelationPkg", true)?;
    let config_path = write_repo_config(temp.path(), &repo_dir, "relation-sample")?;

    let analysis =
        analyze_repository_from_config("relation-sample", Some(&config_path), temp.path())?;
    let mut relations = analysis
        .relations
        .into_iter()
        .map(|relation| {
            json!({
                "kind": format!("{:?}", relation.kind).to_ascii_lowercase(),
                "source_id": relation.source_id,
                "target_id": relation.target_id,
            })
        })
        .collect::<Vec<_>>();
    relations.sort_by(|left, right| {
        left["kind"]
            .as_str()
            .cmp(&right["kind"].as_str())
            .then_with(|| left["source_id"].as_str().cmp(&right["source_id"].as_str()))
            .then_with(|| left["target_id"].as_str().cmp(&right["target_id"].as_str()))
    });

    assert_repo_json_snapshot("repo_relations_result", json!(relations));
    Ok(())
}
