use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::gateway::studio::pathing::{normalize_path_like, studio_display_path};
use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::types::GraphNode;
use crate::gateway::studio::vfs::resolve_navigation_target;
use crate::link_graph::{LinkGraphDirection, LinkGraphIndex};

const DEFAULT_GRAPH_HOPS: usize = 2;
const DEFAULT_GRAPH_LIMIT: usize = 50;
const MAX_GRAPH_HOPS: usize = 8;
const MAX_GRAPH_LIMIT: usize = 300;
pub(super) const LEGACY_NEIGHBOR_LIMIT: usize = 200;

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

pub(super) fn parse_direction(direction: Option<&str>) -> LinkGraphDirection {
    match direction
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("incoming") => LinkGraphDirection::Incoming,
        Some("outgoing") => LinkGraphDirection::Outgoing,
        _ => LinkGraphDirection::Both,
    }
}

pub(super) fn normalize_hops(hops: Option<usize>) -> usize {
    hops.unwrap_or(DEFAULT_GRAPH_HOPS).clamp(1, MAX_GRAPH_HOPS)
}

pub(super) fn normalize_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_GRAPH_LIMIT)
        .clamp(1, MAX_GRAPH_LIMIT)
}

pub(super) fn preferred_label(title: &str, fallback_path: &str) -> String {
    if !title.trim().is_empty() {
        return title.to_string();
    }
    if let Some(stem) = Path::new(fallback_path)
        .file_stem()
        .and_then(|value| value.to_str())
        && !stem.trim().is_empty()
    {
        return stem.to_string();
    }
    fallback_path.to_string()
}

fn resolve_graph_node_id_by_display_path(
    state: &GatewayState,
    index: &LinkGraphIndex,
    node_id: &str,
) -> Option<String> {
    let normalized_target = normalize_path_like(node_id)?;
    for doc in index.docs() {
        let display_path = studio_display_path(state.studio.as_ref(), doc.path.as_str());
        let Some(normalized_display_path) = normalize_path_like(display_path.as_str()) else {
            continue;
        };
        if normalized_display_path == normalized_target {
            return Some(doc.id.clone());
        }
    }
    None
}

pub(super) fn resolve_graph_node_id(
    state: &GatewayState,
    index: &LinkGraphIndex,
    node_id: &str,
) -> Option<String> {
    resolve_graph_node_id_by_display_path(state, index, node_id)
}

pub(super) fn graph_node(
    state: &GatewayState,
    internal_path: &str,
    label: &str,
    is_center: bool,
    distance: usize,
) -> GraphNode {
    let display_path = studio_display_path(state.studio.as_ref(), internal_path);
    let navigation_target = resolve_navigation_target(state.studio.as_ref(), display_path.as_str());
    GraphNode {
        id: display_path.clone(),
        label: preferred_label(label, display_path.as_str()),
        path: display_path,
        navigation_target: Some(navigation_target),
        node_type: "doc".to_string(),
        is_center,
        distance,
    }
}

pub(super) fn sorted_unique_paths(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut items = values
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    items.sort();
    items
}

pub(super) fn topology_position(index: usize, total: usize) -> [f32; 3] {
    if total == 0 {
        return [0.0, 0.0, 0.0];
    }

    let angle = std::f32::consts::TAU * layout_scalar(index) / layout_scalar(total);
    let radius = 14.0 + layout_scalar(index % 7) * 2.5;
    let depth = layout_scalar(index % 9) - 4.0;
    [radius * angle.cos(), radius * angle.sin(), depth]
}

pub(super) fn topology_color(index: usize) -> &'static str {
    const PALETTE: [&str; 8] = [
        "#9ece6a", "#73daca", "#7aa2f7", "#f7768e", "#e0af68", "#bb9af7", "#7dcfff", "#c0caf5",
    ];
    PALETTE[index % PALETTE.len()]
}

pub(super) fn layout_scalar(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}
