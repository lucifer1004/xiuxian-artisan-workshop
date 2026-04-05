//! Integration tests for bounded agentic expansion planning.

#[cfg(feature = "julia")]
use arrow::{
    array::{Array, Float64Array, Int32Array, ListArray, StringArray},
    record_batch::RecordBatch,
};
use std::{collections::HashSet, fs, path::Path};
use tempfile::TempDir;
#[cfg(feature = "julia")]
use xiuxian_wendao::{
    LinkGraphAgenticCandidatePair, LinkGraphAgenticExpansionPlan, RegisteredRepository,
    RepositoryPluginConfig, RepositoryRefreshPolicy,
};
use xiuxian_wendao::{LinkGraphAgenticExpansionConfig, LinkGraphIndex};
#[cfg(feature = "julia")]
use xiuxian_wendao_julia::{
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN, GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
    GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN,
    GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
    build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates,
    build_graph_structural_keyword_overlap_query_inputs,
    build_graph_structural_keyword_overlap_raw_candidate_inputs,
    fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct AgenticIndexFixture {
    _tmp: TempDir,
    index: LinkGraphIndex,
}

fn write_file(path: &Path, content: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn build_index(root: &Path) -> Result<LinkGraphIndex, Box<dyn std::error::Error>> {
    LinkGraphIndex::build(root).map_err(Box::<dyn std::error::Error>::from)
}

fn build_index_fixture(
    files: &[(&str, &str)],
) -> Result<AgenticIndexFixture, Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    for (relative_path, content) in files {
        write_file(&tmp.path().join(relative_path), content)?;
    }
    let index = build_index(tmp.path())?;
    Ok(AgenticIndexFixture { _tmp: tmp, index })
}

fn expansion_config(
    max_workers: usize,
    max_candidates: usize,
    max_pairs_per_worker: usize,
) -> LinkGraphAgenticExpansionConfig {
    LinkGraphAgenticExpansionConfig {
        max_workers,
        max_candidates,
        max_pairs_per_worker,
        time_budget_ms: 1_000.0,
    }
}

#[cfg(feature = "julia")]
fn first_worker_pair(plan: &LinkGraphAgenticExpansionPlan) -> &LinkGraphAgenticCandidatePair {
    let Some(worker) = plan.workers.first() else {
        panic!("agentic expansion plan should include at least one worker");
    };
    let Some(pair) = worker.pairs.first() else {
        panic!("agentic expansion plan should include at least one pair");
    };
    pair
}

#[cfg(feature = "julia")]
fn build_pair_rerank_request_batch(
    index: &LinkGraphIndex,
    pair: &LinkGraphAgenticCandidatePair,
) -> Result<RecordBatch, Box<dyn std::error::Error>> {
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
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    pair.left_id.clone(),
                    pair.right_id.clone(),
                    Vec::new(),
                    left.tags.clone(),
                    right.tags.clone(),
                ),
                pair.priority,
                0.0,
                true,
            )],
        )?;
    Ok(batch)
}

#[cfg(feature = "julia")]
fn required_column<'a, T: Array + 'static>(
    batch: &'a RecordBatch,
    column_name: &str,
    expected_type: &str,
) -> &'a T {
    let Some(column) = batch.column_by_name(column_name) else {
        panic!("`{column_name}` column should exist");
    };
    let Some(column) = column.as_any().downcast_ref::<T>() else {
        panic!("`{column_name}` column should be {expected_type}");
    };
    column
}

#[test]
fn test_agentic_expansion_plan_respects_worker_and_pair_budgets() -> TestResult {
    let fixture = build_index_fixture(&[
        ("notes/a.md", "---\ntags:\n  - alpha\n---\n# A\n\ncontent\n"),
        ("notes/b.md", "---\ntags:\n  - alpha\n---\n# B\n\ncontent\n"),
        ("notes/c.md", "---\ntags:\n  - beta\n---\n# C\n\ncontent\n"),
        ("notes/d.md", "---\ntags:\n  - gamma\n---\n# D\n\ncontent\n"),
    ])?;
    let index = &fixture.index;
    let plan = index.agentic_expansion_plan_with_config(None, expansion_config(2, 4, 2));

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

    let mut seen_pairs = HashSet::<(String, String)>::new();
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
fn test_agentic_expansion_plan_query_narrows_candidates() -> TestResult {
    let fixture = build_index_fixture(&[
        ("docs/a.md", "# A\n\nalpha momentum\n"),
        ("docs/b.md", "# B\n\nalpha breakout\n"),
        ("docs/c.md", "# C\n\nbeta mean reversion\n"),
        ("docs/d.md", "# D\n\ngamma divergence\n"),
    ])?;
    let index = &fixture.index;
    let plan = index.agentic_expansion_plan_with_config(Some("alpha"), expansion_config(3, 10, 3));

    assert_eq!(plan.query.as_deref(), Some("alpha"));
    assert!(plan.candidate_notes <= 2);
    assert!(plan.selected_pairs <= 1);
    assert!(plan.workers.len() <= 1);

    Ok(())
}

#[cfg(feature = "julia")]
#[test]
fn test_agentic_expansion_pair_projects_into_julia_graph_structural_request() -> TestResult {
    let fixture = build_index_fixture(&[
        (
            "notes/a.md",
            "---\ntags:\n  - alpha\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - alpha\n---\n# B\n\nalpha breakout\n",
        ),
        (
            "notes/c.md",
            "---\ntags:\n  - beta\n---\n# C\n\nbeta mean reversion\n",
        ),
        (
            "notes/d.md",
            "---\ntags:\n  - gamma\n---\n# D\n\ngamma divergence\n",
        ),
    ])?;
    let index = &fixture.index;
    let plan = index.agentic_expansion_plan_with_config(Some("alpha"), expansion_config(2, 4, 2));

    let pair = first_worker_pair(&plan);
    let batch = build_pair_rerank_request_batch(index, pair)?;

    let query_ids =
        required_column::<StringArray>(&batch, GRAPH_STRUCTURAL_QUERY_ID_COLUMN, "utf8");
    let retrieval_layers =
        required_column::<Int32Array>(&batch, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN, "int32");
    let query_max_layers =
        required_column::<Int32Array>(&batch, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, "int32");
    let semantic_scores =
        required_column::<Float64Array>(&batch, GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN, "float64");
    let keyword_scores =
        required_column::<Float64Array>(&batch, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN, "float64");
    let tag_scores =
        required_column::<Float64Array>(&batch, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN, "float64");
    let anchor_planes =
        required_column::<ListArray>(&batch, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, "list");
    let anchor_values =
        required_column::<ListArray>(&batch, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN, "list");
    let candidate_node_ids =
        required_column::<ListArray>(&batch, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN, "list");
    let candidate_edge_kinds =
        required_column::<ListArray>(&batch, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, "list");

    let anchor_plane_values = anchor_planes.value(0);
    let Some(anchor_plane_values) = anchor_plane_values.as_any().downcast_ref::<StringArray>()
    else {
        panic!("anchor plane values should be utf8");
    };
    let anchor_value_values = anchor_values.value(0);
    let Some(anchor_value_values) = anchor_value_values.as_any().downcast_ref::<StringArray>()
    else {
        panic!("anchor values should be utf8");
    };
    let candidate_node_values = candidate_node_ids.value(0);
    let Some(candidate_node_values) = candidate_node_values.as_any().downcast_ref::<StringArray>()
    else {
        panic!("candidate node ids should be utf8");
    };

    assert_eq!(query_ids.value(0), "agentic-query-alpha");
    assert_eq!(retrieval_layers.value(0), 0);
    assert_eq!(query_max_layers.value(0), 1);
    assert_eq!(anchor_plane_values.value(0), "keyword");
    assert_eq!(anchor_value_values.value(0), "alpha");
    assert_eq!(candidate_node_ids.value_length(0), 2);
    assert_eq!(candidate_edge_kinds.value_length(0), 0);
    assert_eq!(candidate_node_values.value(0), pair.left_id);
    assert!(semantic_scores.value(0) > 0.0);
    assert!((keyword_scores.value(0) - 1.0).abs() < f64::EPSILON);
    assert!((tag_scores.value(0) - 1.0).abs() < f64::EPSILON);
    assert_eq!(batch.num_rows(), 1);

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
async fn test_agentic_expansion_pair_uses_julia_graph_structural_fetch_helper() -> TestResult {
    let fixture = build_index_fixture(&[
        (
            "notes/a.md",
            "---\ntags:\n  - alpha\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - alpha\n---\n# B\n\nalpha breakout\n",
        ),
        (
            "notes/c.md",
            "---\ntags:\n  - beta\n---\n# C\n\nbeta mean reversion\n",
        ),
        (
            "notes/d.md",
            "---\ntags:\n  - gamma\n---\n# D\n\ngamma divergence\n",
        ),
    ])?;
    let index = &fixture.index;
    let plan = index.agentic_expansion_plan_with_config(Some("alpha"), expansion_config(2, 4, 2));

    let pair = first_worker_pair(&plan);
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

    let Err(error) =
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
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    pair.left_id.clone(),
                    pair.right_id.clone(),
                    Vec::new(),
                    left.tags.clone(),
                    right.tags.clone(),
                ),
                pair.priority,
                0.0,
                true,
            )],
        )
        .await
    else {
        panic!("missing graph-structural transport must fail");
    };

    assert!(
        error.to_string().contains("/graph/structural/rerank"),
        "unexpected error: {error}"
    );

    Ok(())
}
