use xiuxian_wendao_core::repo_intelligence::{
    RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy,
};

use crate::{
    GraphStructuralFilterConstraint, build_graph_structural_generic_topology_candidate_inputs,
    build_graph_structural_generic_topology_candidate_metadata_inputs,
    build_graph_structural_generic_topology_filter_request_batch_from_raw_connected_pair_collections,
    build_graph_structural_keyword_tag_query_context,
    build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples,
    julia_plugin_test_support::official_examples::{
        reserve_real_service_port, spawn_real_wendaosearch_solver_demo_multi_route_service,
        spawn_real_wendaosearch_solver_demo_structural_rerank_service,
        wait_for_service_ready_with_attempts,
    },
};

use super::{
    fetch_graph_structural_generic_topology_filter_rows_for_repository_from_raw_connected_pair_collections,
    fetch_graph_structural_generic_topology_rerank_rows_for_repository,
    fetch_graph_structural_generic_topology_rerank_rows_for_repository_from_raw_connected_pair_collections,
};

#[tokio::test]
async fn fetch_graph_structural_generic_topology_rerank_rows_for_repository_rejects_missing_transport()
 {
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

    let error = fetch_graph_structural_generic_topology_rerank_rows_for_repository(
        &repository,
        &build_graph_structural_keyword_tag_query_context(
            "query-generic",
            0,
            2,
            vec!["alpha".to_string()],
            Vec::new(),
            vec!["depends_on".to_string()],
        )
        .expect("generic topology query"),
        &[build_graph_structural_generic_topology_candidate_inputs(
            build_graph_structural_generic_topology_candidate_metadata_inputs(
                "candidate-chain",
                vec![
                    "node-1".to_string(),
                    "node-2".to_string(),
                    "node-3".to_string(),
                ],
                vec!["node-1".to_string(), "node-2".to_string()],
                vec!["node-2".to_string(), "node-3".to_string()],
                vec!["depends_on".to_string(), "depends_on".to_string()],
            ),
            0.8,
            0.5,
            1.0,
            0.0,
        )],
    )
    .await
    .expect_err("missing graph-structural transport must fail");
    assert!(
        error.to_string().contains("/graph/structural/rerank"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn fetch_graph_structural_generic_topology_rerank_rows_for_repository_against_real_wendaosearch_solver_demo_service()
 {
    let port = reserve_real_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let mut service = spawn_real_wendaosearch_solver_demo_structural_rerank_service(port);
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
                    "base_url": base_url,
                    "structural_rerank": {
                        "route": "/graph/structural/rerank",
                        "schema_version": "v0-draft"
                    }
                }
            }),
        }],
    };

    wait_for_service_ready_with_attempts(&format!("http://127.0.0.1:{port}"), 600)
        .await
        .unwrap_or_else(|error| {
            panic!("wait for real WendaoSearch solver-demo structural Flight service: {error}")
        });

    let candidate_id = "candidate-chain-live".to_string();
    let rows =
        fetch_graph_structural_generic_topology_rerank_rows_for_repository_from_raw_connected_pair_collections(
            &repository,
            &build_graph_structural_keyword_tag_query_context(
                "query-live-generic",
                0,
                2,
                vec!["alpha".to_string()],
                Vec::new(),
                vec!["depends_on".to_string()],
            )
            .expect("generic topology query"),
            &[build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
                candidate_id.clone(),
                vec![("node-1", "node-2", 0.6), ("node-2", "node-3", 0.8)],
                "depends_on",
                0.6,
                1.0,
                0.0,
            )
            .expect("raw connected pair collection candidate")],
        )
        .await
        .unwrap_or_else(|error| {
            panic!("real WendaoSearch solver-demo generic-topology rerank should succeed: {error}")
        });

    let row = rows.get(&candidate_id).unwrap_or_else(|| {
        panic!("missing candidate `{candidate_id}` in solver-demo generic live response")
    });
    assert_eq!(row.candidate_id, candidate_id);
    assert!(row.feasible);
    assert!(row.structural_score > 0.0);
    assert!(row.final_score > row.structural_score);
    assert_eq!(row.pin_assignment, vec!["node-1".to_string()]);
    assert!(
        row.explanation.contains("with 3 nodes, 2 explicit edges"),
        "unexpected explanation: {}",
        row.explanation
    );

    service.kill();
}

#[tokio::test]
async fn fetch_graph_structural_generic_topology_rerank_rows_for_repository_via_manifest_discovery_against_real_wendaosearch_solver_demo_multi_route_service()
 {
    let port = reserve_real_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let mut service = spawn_real_wendaosearch_solver_demo_multi_route_service(port);
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
                    "base_url": base_url,
                    "route": "/plugin/capabilities",
                    "schema_version": "v0-draft"
                }
            }),
        }],
    };

    wait_for_service_ready_with_attempts(&format!("http://127.0.0.1:{port}"), 600)
        .await
        .unwrap_or_else(|error| {
            panic!("wait for real WendaoSearch solver-demo multi-route Flight service: {error}")
        });

    let candidate_id = "candidate-chain-live".to_string();
    let rows =
        fetch_graph_structural_generic_topology_rerank_rows_for_repository_from_raw_connected_pair_collections(
            &repository,
            &build_graph_structural_keyword_tag_query_context(
                "query-live-generic",
                0,
                2,
                vec!["alpha".to_string()],
                Vec::new(),
                vec!["depends_on".to_string()],
            )
            .expect("generic topology query"),
            &[build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
                candidate_id.clone(),
                vec![("node-1", "node-2", 0.6), ("node-2", "node-3", 0.8)],
                "depends_on",
                0.6,
                1.0,
                0.0,
            )
            .expect("raw connected pair collection candidate")],
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "manifest-discovered real WendaoSearch solver-demo generic-topology rerank should succeed: {error}"
            )
        });

    let row = rows.get(&candidate_id).unwrap_or_else(|| {
        panic!("missing candidate `{candidate_id}` in solver-demo generic multi-route response")
    });
    assert_eq!(row.candidate_id, candidate_id);
    assert!(row.feasible);
    assert!(row.structural_score > 0.0);
    assert!(row.final_score > row.structural_score);
    assert_eq!(row.pin_assignment, vec!["node-1".to_string()]);
    assert!(
        row.explanation.contains("with 3 nodes, 2 explicit edges"),
        "unexpected explanation: {}",
        row.explanation
    );

    service.kill();
}

#[tokio::test]
async fn fetch_graph_structural_generic_topology_rerank_rows_for_repository_with_multiple_candidates_via_manifest_discovery_against_real_wendaosearch_solver_demo_multi_route_service()
 {
    let port = reserve_real_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let mut service = spawn_real_wendaosearch_solver_demo_multi_route_service(port);
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
                    "base_url": base_url,
                    "route": "/plugin/capabilities",
                    "schema_version": "v0-draft"
                }
            }),
        }],
    };

    wait_for_service_ready_with_attempts(&format!("http://127.0.0.1:{port}"), 600)
        .await
        .unwrap_or_else(|error| {
            panic!("wait for real WendaoSearch solver-demo multi-route Flight service: {error}")
        });

    let rows =
        fetch_graph_structural_generic_topology_rerank_rows_for_repository_from_raw_connected_pair_collections(
            &repository,
            &build_graph_structural_keyword_tag_query_context(
                "query-live-generic-batch",
                0,
                2,
                vec!["alpha".to_string()],
                Vec::new(),
                vec!["depends_on".to_string()],
            )
            .expect("generic topology query"),
            &[
                build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
                    "candidate-chain-live-a",
                    vec![("node-1", "node-2", 0.6), ("node-2", "node-3", 0.8)],
                    "depends_on",
                    0.6,
                    1.0,
                    0.0,
                )
                .expect("raw connected pair collection candidate"),
                build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
                    "candidate-chain-live-b",
                    vec![("node-4", "node-5", 0.55), ("node-5", "node-6", 0.75)],
                    "depends_on",
                    0.5,
                    1.0,
                    0.0,
                )
                .expect("raw connected pair collection candidate"),
            ],
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "manifest-discovered real WendaoSearch solver-demo multi-candidate generic-topology rerank should succeed: {error}"
            )
        });

    assert_eq!(rows.len(), 2);
    for candidate_id in ["candidate-chain-live-a", "candidate-chain-live-b"] {
        let row = rows.get(candidate_id).unwrap_or_else(|| {
            panic!("missing candidate `{candidate_id}` in solver-demo generic multi-route response")
        });
        assert_eq!(row.candidate_id, candidate_id);
        assert!(row.feasible);
        assert!(row.structural_score > 0.0);
        assert!(row.final_score > row.structural_score);
        assert_eq!(row.pin_assignment.len(), 1);
        assert!(
            row.explanation.contains("with 3 nodes, 2 explicit edges"),
            "unexpected explanation for `{candidate_id}`: {}",
            row.explanation
        );
    }

    service.kill();
}

#[tokio::test]
async fn fetch_graph_structural_generic_topology_filter_rows_for_repository_rejects_missing_transport()
 {
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

    let query = build_graph_structural_keyword_tag_query_context(
        "query-generic-filter",
        0,
        2,
        vec!["alpha".to_string()],
        Vec::new(),
        vec!["depends_on".to_string()],
    )
    .expect("generic topology filter query");
    let constraint =
        GraphStructuralFilterConstraint::new("pin_assignment", 1).expect("filter constraint");
    let candidates = [
        build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
            "candidate-chain-filter",
            vec![("node-1", "node-2", 0.6), ("node-2", "node-3", 0.8)],
            "depends_on",
            0.6,
            1.0,
            0.0,
        )
        .expect("raw connected pair collection candidate"),
    ];

    let error = fetch_graph_structural_generic_topology_filter_rows_for_repository_from_raw_connected_pair_collections(
        &repository,
        &query,
        &constraint,
        &candidates,
    )
    .await
    .expect_err("missing graph-structural filter transport must fail");
    assert!(
        error.to_string().contains("/graph/structural/filter"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn fetch_graph_structural_generic_topology_filter_rows_for_repository_via_manifest_discovery_against_real_wendaosearch_solver_demo_multi_route_service()
 {
    let port = reserve_real_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let mut service = spawn_real_wendaosearch_solver_demo_multi_route_service(port);
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
                    "base_url": base_url,
                    "route": "/plugin/capabilities",
                    "schema_version": "v0-draft"
                }
            }),
        }],
    };

    wait_for_service_ready_with_attempts(&format!("http://127.0.0.1:{port}"), 600)
        .await
        .unwrap_or_else(|error| {
            panic!("wait for real WendaoSearch solver-demo multi-route Flight service: {error}")
        });

    let query = build_graph_structural_keyword_tag_query_context(
        "query-live-generic-filter",
        0,
        2,
        vec!["alpha".to_string()],
        Vec::new(),
        vec!["depends_on".to_string()],
    )
    .expect("generic topology filter query");
    let constraint =
        GraphStructuralFilterConstraint::new("pin_assignment", 1).expect("filter constraint");
    let candidate_id = "candidate-chain-filter-live".to_string();
    let candidates = [
        build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
            candidate_id.clone(),
            vec![("node-1", "node-2", 0.6), ("node-2", "node-3", 0.8)],
            "depends_on",
            0.6,
            1.0,
            0.0,
        )
        .expect("raw connected pair collection candidate"),
    ];

    let request_batch =
        build_graph_structural_generic_topology_filter_request_batch_from_raw_connected_pair_collections(
            &query,
            &constraint,
            &candidates,
        )
        .expect("generic topology filter request batch");
    assert_eq!(request_batch.num_rows(), 1);

    let rows = fetch_graph_structural_generic_topology_filter_rows_for_repository_from_raw_connected_pair_collections(
        &repository,
        &query,
        &constraint,
        &candidates,
    )
    .await
    .unwrap_or_else(|error| {
        panic!(
            "manifest-discovered real WendaoSearch solver-demo generic-topology filter should succeed: {error}"
        )
    });

    let row = rows.get(&candidate_id).unwrap_or_else(|| {
        panic!("missing candidate `{candidate_id}` in solver-demo generic filter response")
    });
    assert_eq!(row.candidate_id, candidate_id);
    assert!(row.accepted);
    assert!(row.structural_score > 0.0);
    assert_eq!(row.pin_assignment, vec!["node-1".to_string()]);
    assert_eq!(row.rejection_reason, "");

    service.kill();
}
