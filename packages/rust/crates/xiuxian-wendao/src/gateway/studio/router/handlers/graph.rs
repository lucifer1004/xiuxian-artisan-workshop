//! Graph intelligence and visualization endpoints for Studio API.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use serde::Deserialize;

use crate::gateway::studio::pathing::studio_display_path;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{
    GraphNeighborsResult, GraphNode, StudioNavigationTarget, Topology3dPayload,
};
use crate::link_graph::LinkGraphDirection;

/// Query parameters for graph-neighbor traversal.
#[derive(Debug, Deserialize)]
pub struct GraphNeighborsQuery {
    /// Optional direction override for neighbor traversal.
    pub direction: Option<String>,
    /// Optional maximum hop distance.
    pub hops: Option<usize>,
    /// Optional maximum number of neighbors to return.
    pub limit: Option<usize>,
}

/// Gets node neighbors.
pub async fn node_neighbors(
    State(state): State<Arc<GatewayState>>,
    AxumPath(node_id): AxumPath<String>,
) -> Result<Json<GraphNeighborsResult>, StudioApiError> {
    let index = state.link_graph_index().await?;
    let neighbors = index.neighbors(node_id.as_str(), LinkGraphDirection::Both, 1, 100);

    let mut nodes = Vec::new();
    let edges = Vec::new();

    for neighbor in neighbors {
        let stem = &neighbor.stem;
        nodes.push(GraphNode {
            id: stem.clone(),
            label: if neighbor.title.is_empty() {
                stem.clone()
            } else {
                neighbor.title.clone()
            },
            path: neighbor.path.clone(),
            navigation_target: StudioNavigationTarget {
                path: studio_display_path(state.studio.as_ref(), neighbor.path.as_str()),
                category: "doc".to_string(),
                project_name: None,
                root_label: None,
                line: None,
                line_end: None,
                column: None,
            },
            node_type: "note".to_string(),
            is_center: *stem == node_id,
            distance: neighbor.distance,
        });
    }

    Ok(Json(GraphNeighborsResult { nodes, edges }))
}

/// Gets graph neighbors.
pub async fn graph_neighbors(
    State(state): State<Arc<GatewayState>>,
    AxumPath(node_id): AxumPath<String>,
) -> Result<Json<GraphNeighborsResult>, StudioApiError> {
    node_neighbors(State(state), AxumPath(node_id)).await
}

/// Gets 3D topology.
pub async fn topology_3d(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<Topology3dPayload>, StudioApiError> {
    let _index = state.link_graph_index().await?;
    Ok(Json(Topology3dPayload {
        nodes: Vec::new(),
        links: Vec::new(),
    }))
}
