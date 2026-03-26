use std::sync::Arc;

use crate::gateway::studio::router::{
    GatewayState, StudioState,
    handlers::graph::{GraphNeighborsQuery, graph_neighbors},
};
use crate::gateway::studio::types::{GraphNeighborsResponse, UiConfig, UiProjectConfig};
use axum::extract::{Path as AxumPath, Query, State};
use serde_json::json;
use tempfile::TempDir;

pub(crate) struct Fixture {
    pub(crate) state: Arc<GatewayState>,
    pub(crate) _temp_dir: TempDir,
}

pub(crate) fn build_fixture_with_projects(
    docs: &[(&str, &str)],
    projects: Vec<UiProjectConfig>,
) -> Fixture {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("create tempdir: {error}"));
    for (path, content) in docs {
        let absolute_path = temp_dir.path().join(path);
        if let Some(parent) = absolute_path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create fixture doc parent: {error}"));
        }
        std::fs::write(absolute_path, content)
            .unwrap_or_else(|error| panic!("write fixture doc: {error}"));
    }

    let mut studio_state = StudioState::new();
    studio_state.project_root = temp_dir.path().to_path_buf();
    studio_state.config_root = temp_dir.path().to_path_buf();
    studio_state.set_ui_config(UiConfig {
        projects,
        repo_projects: Vec::new(),
    });

    Fixture {
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio_state),
        }),
        _temp_dir: temp_dir,
    }
}

pub(crate) fn build_fixture(docs: &[(&str, &str)]) -> Fixture {
    build_fixture_with_projects(
        docs,
        vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }],
    )
}

pub(crate) async fn graph_neighbors_response(
    fixture: &Fixture,
    node_id: &str,
    hops: usize,
    limit: usize,
) -> GraphNeighborsResponse {
    graph_neighbors(
        State(Arc::clone(&fixture.state)),
        AxumPath(node_id.to_string()),
        Query(GraphNeighborsQuery {
            direction: Some("both".to_string()),
            hops: Some(hops),
            limit: Some(limit),
        }),
    )
    .await
    .unwrap_or_else(|error| panic!("graph neighbors should succeed: {error:?}"))
    .0
}

pub(crate) fn assert_graph_neighbors_include_path(response: &GraphNeighborsResponse, suffix: &str) {
    assert!(
        response.nodes.iter().any(|node| node.path.contains(suffix)),
        "expected {suffix} to be present in graph neighbors",
    );
}

pub(crate) fn assert_graph_neighbors_include_link_target(
    response: &GraphNeighborsResponse,
    suffix: &str,
) {
    assert!(
        response
            .links
            .iter()
            .any(|link| link.target.contains(suffix)),
        "expected graph links to point at {suffix}",
    );
}

pub(crate) fn graph_neighbors_snapshot_payload(
    response: GraphNeighborsResponse,
) -> serde_json::Value {
    json!({
        "center": {
            "distance": response.center.distance,
            "id": response.center.id,
            "isCenter": response.center.is_center,
            "label": response.center.label,
            "nodeType": response.center.node_type,
            "path": response.center.path,
        },
        "links": response.links.into_iter().map(|link| {
            json!({
                "direction": link.direction,
                "distance": link.distance,
                "source": link.source,
                "target": link.target,
            })
        }).collect::<Vec<_>>(),
        "nodes": response.nodes.into_iter().map(|node| {
            json!({
                "distance": node.distance,
                "id": node.id,
                "isCenter": node.is_center,
                "label": node.label,
                "nodeType": node.node_type,
                "path": node.path,
            })
        }).collect::<Vec<_>>(),
        "totalLinks": response.total_links,
        "totalNodes": response.total_nodes,
    })
}
