//! Integration tests for bounded agentic expansion planning.

use arrow::array::{Float64Array, Int32Array, ListArray, StringArray};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xiuxian_wendao::analyzers::languages::{
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN, GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
    GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN,
    GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates,
    build_graph_structural_keyword_overlap_query_inputs,
    build_graph_structural_keyword_overlap_raw_candidate_inputs,
    fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates,
};
use xiuxian_wendao::{
    LinkGraphAgenticExpansionConfig, LinkGraphIndex, RegisteredRepository, RepositoryPluginConfig,
    RepositoryRefreshPolicy,
};

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[test]
fn test_agentic_expansion_plan_respects_worker_and_pair_budgets()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("notes/a.md"),
        "---\ntags:\n  - alpha\n---\n# A\n\ncontent\n",
    )?;
    write_file(
        &tmp.path().join("notes/b.md"),
        "---\ntags:\n  - alpha\n---\n# B\n\ncontent\n",
    )?;
    write_file(
        &tmp.path().join("notes/c.md"),
        "---\ntags:\n  - beta\n---\n# C\n\ncontent\n",
    )?;
    write_file(
        &tmp.path().join("notes/d.md"),
        "---\ntags:\n  - gamma\n---\n# D\n\ncontent\n",
    )?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|err| err.to_string())?;
    let plan = index.agentic_expansion_plan_with_config(
        None,
        LinkGraphAgenticExpansionConfig {
            max_workers: 2,
            max_candidates: 4,
            max_pairs_per_worker: 2,
            time_budget_ms: 1_000.0,
        },
    );

    assert_eq!(plan.total_notes, 4);
    assert_eq!(plan.candidate_notes, 4);
    assert_eq!(plan.total_possible_pairs, 6);
    assert!(plan.workers.len() <= 2);
    assert!(plan.workers.iter().all(|worker| worker.pair_count <= 2));
    assert!(plan.selected_pairs <= 4);
    assert_eq!(
        plan.selected_pairs,
        plan.workers
            .iter()
            .map(|worker| worker.pair_count)
            .sum::<usize>()
    );

    let mut seen_pairs = std::collections::HashSet::<(String, String)>::new();
    for worker in &plan.workers {
        for pair in &worker.pairs {
            let key = if pair.left_id <= pair.right_id {
                (pair.left_id.clone(), pair.right_id.clone())
            } else {
                (pair.right_id.clone(), pair.left_id.clone())
            };
            assert!(seen_pairs.insert(key), "duplicate candidate pair in plan");
        }
    }

    Ok(())
}

#[test]
fn test_agentic_expansion_plan_query_narrows_candidates() -> Result<(), Box<dyn std::error::Error>>
{
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha momentum\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nalpha breakout\n")?;
    write_file(
        &tmp.path().join("docs/c.md"),
        "# C\n\nbeta mean reversion\n",
    )?;
    write_file(&tmp.path().join("docs/d.md"), "# D\n\ngamma divergence\n")?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|err| err.to_string())?;
    let plan = index.agentic_expansion_plan_with_config(
        Some("alpha"),
        LinkGraphAgenticExpansionConfig {
            max_workers: 3,
            max_candidates: 10,
            max_pairs_per_worker: 3,
            time_budget_ms: 1_000.0,
        },
    );

    assert_eq!(plan.query.as_deref(), Some("alpha"));
    assert!(plan.candidate_notes <= 2);
    assert!(plan.selected_pairs <= 1);
    assert!(plan.workers.len() <= 1);

    Ok(())
}

#[test]
fn test_agentic_expansion_pair_projects_into_julia_graph_structural_request()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("notes/a.md"),
        "---\ntags:\n  - alpha\n---\n# A\n\nalpha momentum\n",
    )?;
    write_file(
        &tmp.path().join("notes/b.md"),
        "---\ntags:\n  - alpha\n---\n# B\n\nalpha breakout\n",
    )?;
    write_file(
        &tmp.path().join("notes/c.md"),
        "---\ntags:\n  - beta\n---\n# C\n\nbeta mean reversion\n",
    )?;
    write_file(
        &tmp.path().join("notes/d.md"),
        "---\ntags:\n  - gamma\n---\n# D\n\ngamma divergence\n",
    )?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|err| err.to_string())?;
    let plan = index.agentic_expansion_plan_with_config(
        Some("alpha"),
        LinkGraphAgenticExpansionConfig {
            max_workers: 2,
            max_candidates: 4,
            max_pairs_per_worker: 2,
            time_budget_ms: 1_000.0,
        },
    );

    let pair = &plan.workers[0].pairs[0];
    let left = index
        .metadata(&pair.left_id)
        .ok_or_else(|| format!("missing metadata for `{}`", pair.left_id))?;
    let right = index
        .metadata(&pair.right_id)
        .ok_or_else(|| format!("missing metadata for `{}`", pair.right_id))?;
    let batch =
        build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates(
            &build_graph_structural_keyword_overlap_query_inputs(
                "agentic-query-alpha",
                0,
                1,
                vec!["alpha".to_string()],
                Vec::new(),
            ),
            &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                pair.left_id.clone(),
                pair.right_id.clone(),
                Vec::new(),
                left.tags.clone(),
                right.tags.clone(),
                pair.priority,
                0.0,
                true,
            )],
        )?;

    let query_ids = batch
        .column_by_name(GRAPH_STRUCTURAL_QUERY_ID_COLUMN)
        .expect("query_id column should exist")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("query_id column should be utf8");
    let retrieval_layers = batch
        .column_by_name(GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN)
        .expect("retrieval_layer column should exist")
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("retrieval_layer column should be int32");
    let query_max_layers = batch
        .column_by_name(GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN)
        .expect("query_max_layers column should exist")
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("query_max_layers column should be int32");
    let semantic_scores = batch
        .column_by_name(GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN)
        .expect("semantic_score column should exist")
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("semantic_score column should be float64");
    let keyword_scores = batch
        .column_by_name(GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN)
        .expect("keyword_score column should exist")
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("keyword_score column should be float64");
    let tag_scores = batch
        .column_by_name(GRAPH_STRUCTURAL_TAG_SCORE_COLUMN)
        .expect("tag_score column should exist")
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("tag_score column should be float64");
    let anchor_planes = batch
        .column_by_name(GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN)
        .expect("anchor_planes column should exist")
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("anchor_planes column should be list");
    let anchor_values = batch
        .column_by_name(GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN)
        .expect("anchor_values column should exist")
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("anchor_values column should be list");
    let candidate_node_ids = batch
        .column_by_name(GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN)
        .expect("candidate_node_ids column should exist")
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("candidate_node_ids column should be list");
    let candidate_edge_kinds = batch
        .column_by_name(GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN)
        .expect("candidate_edge_kinds column should exist")
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("candidate_edge_kinds column should be list");

    let anchor_plane_values = anchor_planes.value(0);
    let anchor_plane_values = anchor_plane_values
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("anchor plane values should be utf8");
    let anchor_value_values = anchor_values.value(0);
    let anchor_value_values = anchor_value_values
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("anchor values should be utf8");
    let candidate_node_values = candidate_node_ids.value(0);
    let candidate_node_values = candidate_node_values
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("candidate node ids should be utf8");

    assert_eq!(query_ids.value(0), "agentic-query-alpha");
    assert_eq!(retrieval_layers.value(0), 0);
    assert_eq!(query_max_layers.value(0), 1);
    assert_eq!(anchor_plane_values.value(0), "keyword");
    assert_eq!(anchor_value_values.value(0), "alpha");
    assert_eq!(candidate_node_ids.value_length(0), 2);
    assert_eq!(candidate_edge_kinds.value_length(0), 0);
    assert_eq!(candidate_node_values.value(0), pair.left_id);
    assert!(semantic_scores.value(0) > 0.0);
    assert_eq!(keyword_scores.value(0), 1.0);
    assert_eq!(tag_scores.value(0), 1.0);
    assert_eq!(batch.num_rows(), 1);

    Ok(())
}

#[tokio::test]
async fn test_agentic_expansion_pair_uses_julia_graph_structural_fetch_helper()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("notes/a.md"),
        "---\ntags:\n  - alpha\n---\n# A\n\nalpha momentum\n",
    )?;
    write_file(
        &tmp.path().join("notes/b.md"),
        "---\ntags:\n  - alpha\n---\n# B\n\nalpha breakout\n",
    )?;
    write_file(
        &tmp.path().join("notes/c.md"),
        "---\ntags:\n  - beta\n---\n# C\n\nbeta mean reversion\n",
    )?;
    write_file(
        &tmp.path().join("notes/d.md"),
        "---\ntags:\n  - gamma\n---\n# D\n\ngamma divergence\n",
    )?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|err| err.to_string())?;
    let plan = index.agentic_expansion_plan_with_config(
        Some("alpha"),
        LinkGraphAgenticExpansionConfig {
            max_workers: 2,
            max_candidates: 4,
            max_pairs_per_worker: 2,
            time_budget_ms: 1_000.0,
        },
    );

    let pair = &plan.workers[0].pairs[0];
    let left = index
        .metadata(&pair.left_id)
        .ok_or_else(|| format!("missing metadata for `{}`", pair.left_id))?;
    let right = index
        .metadata(&pair.right_id)
        .ok_or_else(|| format!("missing metadata for `{}`", pair.right_id))?;
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        path: None,
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({}),
        }],
    };

    let error =
        fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates(
            &repository,
            &build_graph_structural_keyword_overlap_query_inputs(
                "agentic-query-alpha",
                0,
                1,
                vec!["alpha".to_string()],
                Vec::new(),
            ),
            &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                pair.left_id.clone(),
                pair.right_id.clone(),
                Vec::new(),
                left.tags.clone(),
                right.tags.clone(),
                pair.priority,
                0.0,
                true,
            )],
        )
        .await
        .expect_err("missing graph-structural transport must fail");

    assert!(
        error.to_string().contains("/graph/structural/rerank"),
        "unexpected error: {error}"
    );

    Ok(())
}
