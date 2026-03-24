use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};

use crate::gateway::studio::pathing::studio_display_path;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{GraphLink, GraphNeighborsResponse, GraphNode, NodeNeighbors};
use crate::link_graph::{LinkGraphDirection, LinkGraphIndex, LinkGraphNeighbor};

use super::shared::{
    GraphNeighborsQuery, LEGACY_NEIGHBOR_LIMIT, graph_node, normalize_hops, normalize_limit,
    parse_direction, preferred_label, resolve_graph_node_id, sorted_unique_paths,
};

/// Gets node neighbors.
///
/// # Errors
///
/// Returns an error when the graph index cannot be loaded or when the
/// requested node does not exist.
pub async fn node_neighbors(
    State(state): State<Arc<GatewayState>>,
    AxumPath(node_id): AxumPath<String>,
) -> Result<Json<NodeNeighbors>, StudioApiError> {
    let index = state.link_graph_index().await?;
    let Some(resolved_node_id) =
        resolve_graph_node_id(state.as_ref(), index.as_ref(), node_id.as_str())
    else {
        return Err(StudioApiError::not_found(format!(
            "graph node `{node_id}` was not found"
        )));
    };
    let Some(center) = index.metadata(resolved_node_id.as_str()) else {
        return Err(StudioApiError::not_found(format!(
            "graph node `{node_id}` was not found"
        )));
    };

    let incoming = sorted_unique_paths(
        index
            .neighbors(
                resolved_node_id.as_str(),
                LinkGraphDirection::Incoming,
                1,
                LEGACY_NEIGHBOR_LIMIT,
            )
            .into_iter()
            .map(|neighbor| studio_display_path(state.studio.as_ref(), neighbor.path.as_str())),
    );
    let outgoing = sorted_unique_paths(
        index
            .neighbors(
                resolved_node_id.as_str(),
                LinkGraphDirection::Outgoing,
                1,
                LEGACY_NEIGHBOR_LIMIT,
            )
            .into_iter()
            .map(|neighbor| studio_display_path(state.studio.as_ref(), neighbor.path.as_str())),
    );
    let two_hop = sorted_unique_paths(
        index
            .neighbors(
                resolved_node_id.as_str(),
                LinkGraphDirection::Both,
                2,
                LEGACY_NEIGHBOR_LIMIT.saturating_mul(2),
            )
            .into_iter()
            .filter(|neighbor| neighbor.distance == 2)
            .map(|neighbor| studio_display_path(state.studio.as_ref(), neighbor.path.as_str())),
    );

    Ok(Json(NodeNeighbors {
        node_id: studio_display_path(state.studio.as_ref(), center.path.as_str()),
        name: preferred_label(center.title.as_str(), center.path.as_str()),
        node_type: "doc".to_string(),
        incoming,
        outgoing,
        two_hop,
    }))
}

/// Gets graph neighbors.
///
/// # Errors
///
/// Returns an error when the graph index cannot be loaded or when the
/// requested node does not exist.
pub async fn graph_neighbors(
    State(state): State<Arc<GatewayState>>,
    AxumPath(node_id): AxumPath<String>,
    Query(query): Query<GraphNeighborsQuery>,
) -> Result<Json<GraphNeighborsResponse>, StudioApiError> {
    let index = state.link_graph_index().await?;
    let (resolved_node_id, center_path, center_title) =
        resolve_center_node(state.as_ref(), index.as_ref(), node_id.as_str())?;
    let direction = parse_direction(query.direction.as_deref());
    let hops = normalize_hops(query.hops);
    let limit = normalize_limit(query.limit);
    let neighbors = index.neighbors(resolved_node_id.as_str(), direction, hops, limit);
    let (center_node, mut nodes, internal_paths_by_display_id) = collect_neighbor_nodes(
        state.as_ref(),
        center_path.as_str(),
        center_title.as_str(),
        neighbors.as_slice(),
    );
    let mut links = collect_neighbor_links(
        state.as_ref(),
        index.as_ref(),
        nodes.as_slice(),
        &internal_paths_by_display_id,
    );

    nodes.sort_by(|left, right| {
        right
            .is_center
            .cmp(&left.is_center)
            .then_with(|| left.distance.cmp(&right.distance))
            .then_with(|| left.id.cmp(&right.id))
    });
    links.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.direction.cmp(&right.direction))
    });

    Ok(Json(GraphNeighborsResponse {
        center: center_node,
        total_nodes: nodes.len(),
        total_links: links.len(),
        nodes,
        links,
    }))
}

fn resolve_center_node(
    state: &GatewayState,
    index: &LinkGraphIndex,
    node_id: &str,
) -> Result<(String, String, String), StudioApiError> {
    let Some(resolved_node_id) = resolve_graph_node_id(state, index, node_id) else {
        return Err(graph_node_not_found(node_id));
    };
    let Some(center_metadata) = index.metadata(resolved_node_id.as_str()) else {
        return Err(graph_node_not_found(node_id));
    };
    Ok((
        resolved_node_id,
        center_metadata.path.clone(),
        center_metadata.title.clone(),
    ))
}

fn collect_neighbor_nodes(
    state: &GatewayState,
    center_path: &str,
    center_title: &str,
    neighbors: &[LinkGraphNeighbor],
) -> (GraphNode, Vec<GraphNode>, HashMap<String, String>) {
    let mut nodes = Vec::<GraphNode>::new();
    let mut seen_ids = HashSet::<String>::new();
    let mut internal_paths_by_display_id = HashMap::<String, String>::new();

    let center_node = graph_node(state, center_path, center_title, true, 0);
    internal_paths_by_display_id.insert(center_node.id.clone(), center_path.to_string());
    seen_ids.insert(center_node.id.clone());
    nodes.push(center_node.clone());

    for neighbor in neighbors {
        let node = graph_node(
            state,
            neighbor.path.as_str(),
            neighbor.title.as_str(),
            false,
            neighbor.distance,
        );
        internal_paths_by_display_id.insert(node.id.clone(), neighbor.path.clone());
        if seen_ids.insert(node.id.clone()) {
            nodes.push(node);
        }
    }

    (center_node, nodes, internal_paths_by_display_id)
}

fn collect_neighbor_links(
    state: &GatewayState,
    index: &LinkGraphIndex,
    nodes: &[GraphNode],
    internal_paths_by_display_id: &HashMap<String, String>,
) -> Vec<GraphLink> {
    let included_ids = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let mut links = Vec::<GraphLink>::new();
    let mut seen_links = HashSet::<(String, String)>::new();

    for source in nodes {
        let Some(source_internal_path) = internal_paths_by_display_id.get(&source.id) else {
            continue;
        };
        let edge_limit = index
            .neighbor_count(source_internal_path.as_str(), LinkGraphDirection::Outgoing)
            .max(1);
        for outgoing in index.neighbors(
            source_internal_path.as_str(),
            LinkGraphDirection::Outgoing,
            1,
            edge_limit,
        ) {
            let target = studio_display_path(state.studio.as_ref(), outgoing.path.as_str());
            if source.id == target || !included_ids.contains(target.as_str()) {
                continue;
            }
            let key = (source.id.clone(), target.clone());
            if seen_links.insert(key.clone()) {
                links.push(GraphLink {
                    source: key.0,
                    target: key.1,
                    direction: "outgoing".to_string(),
                    distance: 1,
                });
            }
        }
    }

    links
}

fn graph_node_not_found(node_id: &str) -> StudioApiError {
    StudioApiError::not_found(format!("graph node `{node_id}` was not found"))
}
