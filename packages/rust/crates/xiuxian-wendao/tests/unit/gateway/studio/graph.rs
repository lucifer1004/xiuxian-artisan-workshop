use std::sync::Arc;

use super::*;
use crate::gateway::studio::router::{GatewayState, StudioState};
use serde_json::json;
use tempfile::tempdir;

#[path = "support.rs"]
mod support;
use support::{assert_studio_json_snapshot, round_f32};

struct GraphFixture {
    state: Arc<GatewayState>,
    _temp_dir: tempfile::TempDir,
}

fn make_graph_fixture(docs: Vec<(&str, &str)>) -> GraphFixture {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create graph fixture tempdir: {err}"));
    for (name, content) in docs {
        std::fs::write(temp_dir.path().join(name), content)
            .unwrap_or_else(|err| panic!("failed to write fixture doc {name}: {err}"));
    }

    let mut studio_state = StudioState::new();
    studio_state.project_root = temp_dir.path().to_path_buf();
    studio_state.data_root = temp_dir.path().to_path_buf();
    studio_state.knowledge_root = temp_dir.path().to_path_buf();
    studio_state.internal_skill_root = temp_dir.path().to_path_buf();

    GraphFixture {
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio_state),
        }),
        _temp_dir: temp_dir,
    }
}

#[tokio::test]
async fn node_neighbors_returns_live_neighbors() {
    let fixture = make_graph_fixture(vec![
        ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
        ("beta.md", "# Beta\n\nSee [[gamma]].\n"),
        ("gamma.md", "# Gamma\n\nTail node.\n"),
    ]);

    let result = node_neighbors(fixture.state.as_ref(), "alpha.md").await;
    let Ok(response) = result else {
        panic!("expected node neighbors request to succeed");
    };

    assert_studio_json_snapshot(
        "graph_node_neighbors",
        json!({
            "nodeId": response.node_id,
            "name": response.name,
            "nodeType": response.node_type,
            "incoming": response.incoming,
            "outgoing": response.outgoing,
            "twoHop": response.two_hop,
        }),
    );
}

#[tokio::test]
async fn graph_neighbors_includes_center_node_and_links() {
    let fixture = make_graph_fixture(vec![
        ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
        ("beta.md", "# Beta\n\nBody.\n"),
    ]);

    let result = graph_neighbors(fixture.state.as_ref(), "alpha.md", "both", 2, 10).await;
    let Ok(response) = result else {
        panic!("expected graph neighbors request to succeed");
    };

    let mut nodes = response
        .nodes
        .into_iter()
        .map(|node| {
            json!({
                "id": node.id,
                "label": node.label,
                "path": node.path,
                "nodeType": node.node_type,
                "isCenter": node.is_center,
                "distance": node.distance,
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    let mut links = response
        .links
        .into_iter()
        .map(|link| {
            json!({
                "source": link.source,
                "target": link.target,
                "direction": link.direction,
                "distance": link.distance,
            })
        })
        .collect::<Vec<_>>();
    links.sort_by(|left, right| {
        left["source"]
            .as_str()
            .cmp(&right["source"].as_str())
            .then_with(|| left["target"].as_str().cmp(&right["target"].as_str()))
    });

    assert_studio_json_snapshot(
        "graph_neighbors_payload",
        json!({
            "center": {
                "id": response.center.id,
                "label": response.center.label,
                "path": response.center.path,
                "nodeType": response.center.node_type,
                "isCenter": response.center.is_center,
                "distance": response.center.distance,
            },
            "nodes": nodes,
            "links": links,
            "totalNodes": response.total_nodes,
            "totalLinks": response.total_links,
        }),
    );
}

#[tokio::test]
async fn topology_3d_returns_nodes_and_links() {
    let fixture = make_graph_fixture(vec![
        ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
        ("beta.md", "# Beta\n\nBody.\n"),
    ]);

    let result = topology_3d(fixture.state.as_ref()).await;
    let Ok(response) = result else {
        panic!("expected topology request to succeed");
    };

    let mut nodes = response
        .nodes
        .into_iter()
        .map(|node| {
            json!({
                "id": node.id,
                "name": node.name,
                "nodeType": node.node_type,
                "position": node.position.map(round_f32),
                "clusterId": node.cluster_id,
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    let mut links = response
        .links
        .into_iter()
        .map(|link| {
            json!({
                "from": link.from,
                "to": link.to,
                "label": link.label,
            })
        })
        .collect::<Vec<_>>();
    links.sort_by(|left, right| {
        left["from"]
            .as_str()
            .cmp(&right["from"].as_str())
            .then_with(|| left["to"].as_str().cmp(&right["to"].as_str()))
    });

    let mut clusters = response
        .clusters
        .into_iter()
        .map(|cluster| {
            json!({
                "id": cluster.id,
                "name": cluster.name,
                "centroid": cluster.centroid.map(round_f32),
                "nodeCount": cluster.node_count,
                "color": cluster.color,
            })
        })
        .collect::<Vec<_>>();
    clusters.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    assert_studio_json_snapshot(
        "topology_3d_payload",
        json!({
            "nodes": nodes,
            "links": links,
            "clusters": clusters,
        }),
    );
}

#[tokio::test]
async fn graph_neighbors_returns_not_found_for_unknown_node() {
    let fixture = make_graph_fixture(vec![("alpha.md", "# Alpha\n\nBody.\n")]);

    let result = graph_neighbors(fixture.state.as_ref(), "missing.md", "both", 2, 10).await;
    let Err(error) = result else {
        panic!("expected missing node lookup to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.code(), "NOT_FOUND");
}
