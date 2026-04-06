//! Integration tests for bounded agentic expansion planning.

#[cfg(feature = "julia")]
use arrow::array::{Float64Array, Int32Array, ListArray, StringArray};
use std::{collections::HashSet, fs, path::Path};
use tempfile::TempDir;
use xiuxian_wendao::{LinkGraphAgenticExpansionConfig, LinkGraphIndex};
#[cfg(feature = "julia")]
use xiuxian_wendao::{RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy};
#[cfg(feature = "julia")]
use xiuxian_wendao_builtin::{
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN,
    GRAPH_STRUCTURAL_QUERY_ID_COLUMN, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN,
    GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN, GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN,
    GRAPH_STRUCTURAL_TAG_SCORE_COLUMN, GraphStructuralFilterRequestRow,
    build_graph_structural_filter_request_batch,
    build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs,
    build_graph_structural_keyword_overlap_query_inputs,
    build_graph_structural_keyword_overlap_raw_candidate_inputs,
    fetch_graph_structural_filter_rows_for_repository,
    fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates,
    linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service,
    linked_builtin_spawn_wendaosearch_solver_demo_structural_rerank_service,
};

#[cfg(feature = "julia")]
use super::expansion_support::{
    GenericTopologyCandidateBuildOptions, GenericTopologyCandidateScores,
    assert_solver_demo_generic_topology_row_basics,
    assert_solver_demo_generic_topology_row_infeasible,
    assert_solver_demo_generic_topology_row_shape, build_pair_rerank_request_batch,
    build_raw_connected_pair_collection_candidate_from_pairs,
    build_raw_connected_pair_collection_candidates_from_plan,
    build_raw_seed_centered_pair_collection_candidates_from_plan,
    build_worker_partition_generic_topology_candidate_fixtures_from_plan,
    default_agentic_execution_relation_edge_kind,
    fetch_generic_topology_rows_via_manifest_discovery, first_connected_pair_collection,
    first_worker_pair, required_column,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(feature = "julia")]
#[path = "expansion_plan_batch_tests.rs"]
mod expansion_plan_batch_tests;

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
            "---\ntags:\n  - alpha\n  - core\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - core\n---\n# B\n\nalpha breakout\n",
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
    let candidate_edge_sources = required_column::<ListArray>(
        &batch,
        GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN,
        "list",
    );
    let candidate_edge_destinations = required_column::<ListArray>(
        &batch,
        GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
        "list",
    );
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
    assert_eq!(candidate_edge_sources.value_length(0), 0);
    assert_eq!(candidate_edge_destinations.value_length(0), 0);
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
                2,
                vec!["alpha".to_string()],
                vec!["depends_on".to_string()],
            ),
            &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    pair.left_id.clone(),
                    pair.right_id.clone(),
                    vec!["depends_on".to_string()],
                    left.tags.clone(),
                    right.tags.clone(),
                ),
                0.7,
                0.6,
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

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_fetch_helper_against_solver_demo_service()
-> TestResult {
    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_structural_rerank_service().await;
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        path: None,
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "graph_structural_transport": {
                    "base_url": server_base_url,
                    "structural_rerank": {
                        "route": "/graph/structural/rerank",
                        "schema_version": "v0-draft"
                    }
                }
            }),
        }],
    };

    let rows =
        fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates(
            &repository,
            &build_graph_structural_keyword_overlap_query_inputs(
                "query-live",
                0,
                2,
                vec!["alpha".to_string()],
                vec!["depends_on".to_string()],
            ),
            &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    "node-1".to_string(),
                    "node-2".to_string(),
                    vec!["depends_on".to_string()],
                    vec!["alpha".to_string(), "core".to_string()],
                    vec!["core".to_string()],
                ),
                0.7,
                0.6,
                true,
            )],
        )
        .await?;
    server_guard.kill();

    assert_eq!(rows.len(), 1);
    let row = rows
        .values()
        .next()
        .ok_or_else(|| "missing solver_demo structural-rerank row".to_string())?;
    assert!(
        row.feasible,
        "unexpected explicit solver_demo row: candidate_id={} structural_score={} final_score={} explanation={} pin_assignment={:?}",
        row.candidate_id,
        row.structural_score,
        row.final_score,
        row.explanation,
        row.pin_assignment
    );
    assert!(
        row.structural_score > 0.0,
        "unexpected explicit solver_demo structural_score: {}",
        row.structural_score
    );
    assert!(
        row.final_score > row.structural_score,
        "unexpected explicit solver_demo final_score={} structural_score={}",
        row.final_score,
        row.structural_score
    );
    assert_eq!(row.pin_assignment, vec!["notes/a".to_string()]);
    assert!(
        row.explanation
            .contains("solver_demo feasible candidate via rydberg solve"),
        "unexpected explanation: {}",
        row.explanation
    );

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_fetch_helper_via_manifest_discovery_against_solver_demo_multi_route_service()
-> TestResult {
    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service().await;
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        path: None,
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "capability_manifest_transport": {
                    "base_url": server_base_url,
                    "route": "/plugin/capabilities",
                    "schema_version": "v0-draft"
                }
            }),
        }],
    };

    let rows =
        fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates(
            &repository,
            &build_graph_structural_keyword_overlap_query_inputs(
                "query-live",
                0,
                2,
                vec!["alpha".to_string()],
                vec!["depends_on".to_string()],
            ),
            &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    "node-1".to_string(),
                    "node-2".to_string(),
                    vec!["depends_on".to_string()],
                    vec!["alpha".to_string(), "core".to_string()],
                    vec!["core".to_string()],
                ),
                0.7,
                0.6,
                true,
            )],
        )
        .await?;
    server_guard.kill();

    assert_eq!(rows.len(), 1);
    let row = rows
        .values()
        .next()
        .ok_or_else(|| "missing solver_demo structural-rerank row".to_string())?;
    assert!(
        row.feasible,
        "unexpected manifest solver_demo row: candidate_id={} structural_score={} final_score={} explanation={} pin_assignment={:?}",
        row.candidate_id,
        row.structural_score,
        row.final_score,
        row.explanation,
        row.pin_assignment
    );
    assert!(
        row.structural_score > 0.0,
        "unexpected manifest solver_demo structural_score: {}",
        row.structural_score
    );
    assert!(
        row.final_score > row.structural_score,
        "unexpected manifest solver_demo final_score={} structural_score={}",
        row.final_score,
        row.structural_score
    );
    assert_eq!(row.pin_assignment, vec!["notes/a".to_string()]);
    assert!(
        row.explanation
            .contains("solver_demo feasible candidate via rydberg solve"),
        "unexpected explanation: {}",
        row.explanation
    );

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_filter_helper_via_manifest_discovery_against_solver_demo_multi_route_service()
-> TestResult {
    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service().await;
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        path: None,
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "capability_manifest_transport": {
                    "base_url": server_base_url,
                    "route": "/plugin/capabilities",
                    "schema_version": "v0-draft"
                }
            }),
        }],
    };

    let batch = build_graph_structural_filter_request_batch(&[GraphStructuralFilterRequestRow {
        query_id: "query-live".to_string(),
        candidate_id: "candidate-filter-live".to_string(),
        retrieval_layer: 0,
        query_max_layers: 2,
        constraint_kind: "pin_assignment".to_string(),
        required_boundary_size: 1,
        anchor_planes: vec!["semantic".to_string()],
        anchor_values: vec!["alpha".to_string()],
        edge_constraint_kinds: vec!["depends_on".to_string()],
        candidate_node_ids: vec!["node-1".to_string(), "node-2".to_string()],
        candidate_edge_sources: vec!["node-1".to_string()],
        candidate_edge_destinations: vec!["node-2".to_string()],
        candidate_edge_kinds: vec!["depends_on".to_string()],
    }])?;

    let rows = fetch_graph_structural_filter_rows_for_repository(&repository, &[batch]).await?;
    server_guard.kill();

    assert_eq!(rows.len(), 1);
    let row = rows
        .values()
        .next()
        .ok_or_else(|| "missing solver_demo constraint-filter row".to_string())?;
    assert!(
        row.accepted,
        "unexpected manifest solver_demo filter row: candidate_id={} structural_score={} rejection_reason={} pin_assignment={:?}",
        row.candidate_id, row.structural_score, row.rejection_reason, row.pin_assignment
    );
    assert!(
        row.structural_score > 0.0,
        "unexpected manifest solver_demo filter structural_score: {}",
        row.structural_score
    );
    assert_eq!(row.pin_assignment, vec!["node-1".to_string()]);
    assert_eq!(row.rejection_reason, "");

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_generic_topology_fetch_helper_via_manifest_discovery_against_solver_demo_multi_route_service()
-> TestResult {
    let fixture = build_index_fixture(&[
        (
            "notes/a.md",
            "---\ntags:\n  - alpha\n  - core\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - alpha\n  - bridge\n---\n# B\n\nalpha breakout\n",
        ),
        (
            "notes/c.md",
            "---\ntags:\n  - alpha\n  - leaf\n---\n# C\n\nalpha continuation\n",
        ),
    ])?;
    let plan = fixture
        .index
        .agentic_expansion_plan_with_config(Some("alpha"), expansion_config(1, 10, 10));
    let pair_collection = first_connected_pair_collection(&plan);
    let relation_edge_kind = default_agentic_execution_relation_edge_kind();

    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service().await;
    let candidate_id = "candidate-chain-live".to_string();
    let rows = fetch_generic_topology_rows_via_manifest_discovery(
        server_base_url,
        "query-live-generic",
        &[build_raw_connected_pair_collection_candidate_from_pairs(
            candidate_id.clone(),
            pair_collection.as_slice(),
            &relation_edge_kind,
            0.35,
            1.0,
            1.0,
        )?],
    )
    .await?;
    server_guard.kill();

    assert_eq!(rows.len(), 1);
    let row = rows
        .get(&candidate_id)
        .ok_or_else(|| "missing solver_demo generic-topology row".to_string())?;
    assert_solver_demo_generic_topology_row_basics(row, "single-candidate");
    assert_solver_demo_generic_topology_row_shape(row, "single-candidate", 3, 2);
    assert_eq!(row.pin_assignment, vec!["notes/a".to_string()]);

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_generic_topology_fetch_helper_for_multiple_connected_pair_collections_via_manifest_discovery_against_solver_demo_multi_route_service()
-> TestResult {
    let fixture = build_index_fixture(&[
        (
            "notes/a.md",
            "---\ntags:\n  - alpha\n  - core\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - alpha\n  - bridge\n---\n# B\n\nalpha breakout\n",
        ),
        (
            "notes/c.md",
            "---\ntags:\n  - alpha\n  - leaf\n---\n# C\n\nalpha continuation\n",
        ),
        (
            "notes/d.md",
            "---\ntags:\n  - alpha\n  - branch\n---\n# D\n\nalpha rotation\n",
        ),
    ])?;
    let plan = fixture
        .index
        .agentic_expansion_plan_with_config(Some("alpha"), expansion_config(1, 20, 20));
    let relation_edge_kind = default_agentic_execution_relation_edge_kind();
    let candidate_options = GenericTopologyCandidateBuildOptions::new(
        "candidate-chain-live",
        &relation_edge_kind,
        GenericTopologyCandidateScores::new(0.35, 1.0, 1.0),
    );
    let candidates =
        build_raw_connected_pair_collection_candidates_from_plan(&plan, 2, &candidate_options)?;
    assert_eq!(candidates.len(), 2);

    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service().await;
    let rows = fetch_generic_topology_rows_via_manifest_discovery(
        server_base_url,
        "query-live-generic-batch",
        &candidates,
    )
    .await?;
    server_guard.kill();

    assert_eq!(rows.len(), 2);
    for candidate_id in ["candidate-chain-live-0", "candidate-chain-live-1"] {
        let row = rows
            .get(candidate_id)
            .ok_or_else(|| format!("missing solver_demo generic-topology row `{candidate_id}`"))?;
        assert_solver_demo_generic_topology_row_basics(row, candidate_id);
        assert_solver_demo_generic_topology_row_shape(row, candidate_id, 3, 2);
        assert_eq!(
            row.pin_assignment.len(),
            1,
            "unexpected pin assignment for `{candidate_id}`: {:?}",
            row.pin_assignment
        );
    }

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_generic_topology_fetch_helper_for_seed_centered_plan_candidate_batches_via_manifest_discovery_against_solver_demo_multi_route_service()
-> TestResult {
    let fixture = build_index_fixture(&[
        (
            "notes/a.md",
            "---\ntags:\n  - alpha\n  - core\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - alpha\n  - branch\n---\n# B\n\nalpha breakout\n",
        ),
        (
            "notes/c.md",
            "---\ntags:\n  - alpha\n  - leaf\n---\n# C\n\nalpha continuation\n",
        ),
        (
            "notes/d.md",
            "---\ntags:\n  - alpha\n  - bridge\n---\n# D\n\nalpha rotation\n",
        ),
        (
            "notes/e.md",
            "---\ntags:\n  - alpha\n  - satellite\n---\n# E\n\nalpha expansion\n",
        ),
    ])?;
    let plan = fixture
        .index
        .agentic_expansion_plan_with_config(Some("alpha"), expansion_config(1, 10, 10));
    assert_eq!(plan.selected_pairs, 10);
    let relation_edge_kind = default_agentic_execution_relation_edge_kind();
    let candidate_options = GenericTopologyCandidateBuildOptions::new(
        "candidate-seed-live",
        &relation_edge_kind,
        GenericTopologyCandidateScores::new(0.35, 1.0, 1.0),
    );
    let candidates = build_raw_seed_centered_pair_collection_candidates_from_plan(
        &plan,
        2,
        3,
        &candidate_options,
    )?;
    assert_eq!(candidates.len(), 2);

    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service().await;
    let rows = fetch_generic_topology_rows_via_manifest_discovery(
        server_base_url,
        "query-live-generic-seed-batch",
        &candidates,
    )
    .await?;
    server_guard.kill();

    assert_eq!(rows.len(), 2);
    for candidate_id in ["candidate-seed-live-0", "candidate-seed-live-1"] {
        let row = rows.get(candidate_id).ok_or_else(|| {
            format!("missing solver_demo seed-centered generic-topology row `{candidate_id}`")
        })?;
        assert_solver_demo_generic_topology_row_basics(row, candidate_id);
        assert_solver_demo_generic_topology_row_shape(row, candidate_id, 5, 4);
        assert_eq!(
            row.pin_assignment.len(),
            1,
            "unexpected pin assignment for `{candidate_id}`: {:?}",
            row.pin_assignment
        );
    }

    Ok(())
}

#[cfg(feature = "julia")]
#[tokio::test]
#[serial_test::serial(julia_live)]
async fn test_host_uses_julia_graph_structural_generic_topology_fetch_helper_for_worker_partition_plan_candidate_batches_via_manifest_discovery_against_solver_demo_multi_route_service()
-> TestResult {
    let fixture = build_index_fixture(&[
        (
            "notes/a.md",
            "---\ntags:\n  - alpha\n  - core\n---\n# A\n\nalpha momentum\n",
        ),
        (
            "notes/b.md",
            "---\ntags:\n  - alpha\n  - branch\n---\n# B\n\nalpha breakout\n",
        ),
        (
            "notes/c.md",
            "---\ntags:\n  - alpha\n  - leaf\n---\n# C\n\nalpha continuation\n",
        ),
        (
            "notes/d.md",
            "---\ntags:\n  - alpha\n  - bridge\n---\n# D\n\nalpha rotation\n",
        ),
        (
            "notes/e.md",
            "---\ntags:\n  - alpha\n  - satellite\n---\n# E\n\nalpha expansion\n",
        ),
    ])?;
    let plan = fixture
        .index
        .agentic_expansion_plan_with_config(Some("alpha"), expansion_config(2, 10, 3));
    let relation_edge_kind = default_agentic_execution_relation_edge_kind();
    let candidate_options = GenericTopologyCandidateBuildOptions::new(
        "candidate-worker-live",
        &relation_edge_kind,
        GenericTopologyCandidateScores::new(0.35, 1.0, 1.0),
    );
    let fixtures = build_worker_partition_generic_topology_candidate_fixtures_from_plan(
        &plan,
        2,
        2,
        &candidate_options,
    )?;
    assert_eq!(fixtures.len(), 2);

    let candidates = fixtures
        .iter()
        .map(|fixture| fixture.candidate.clone())
        .collect::<Vec<_>>();

    let (server_base_url, mut server_guard) =
        linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service().await;
    let rows = fetch_generic_topology_rows_via_manifest_discovery(
        server_base_url,
        "query-live-generic-worker-batch",
        &candidates,
    )
    .await?;
    server_guard.kill();

    assert_eq!(rows.len(), fixtures.len());
    assert!(
        rows.values().any(|row| row.feasible),
        "expected at least one feasible worker-partition row, got {:?}",
        rows.keys().collect::<Vec<_>>()
    );
    for fixture in &fixtures {
        let row = rows.get(&fixture.candidate_id).ok_or_else(|| {
            format!(
                "missing solver_demo worker-partition generic-topology row `{}`",
                fixture.candidate_id
            )
        })?;
        if row.feasible {
            assert_solver_demo_generic_topology_row_basics(row, &fixture.candidate_id);
            assert_solver_demo_generic_topology_row_shape(
                row,
                &fixture.candidate_id,
                fixture.expected_nodes,
                fixture.expected_edges,
            );
            assert_eq!(
                row.pin_assignment.len(),
                1,
                "unexpected pin assignment for `{}`: {:?}",
                fixture.candidate_id,
                row.pin_assignment
            );
        } else {
            assert_solver_demo_generic_topology_row_infeasible(row, &fixture.candidate_id);
        }
    }

    Ok(())
}
