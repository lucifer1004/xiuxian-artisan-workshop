use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::gateway::studio::pathing::studio_display_path;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{
    Topology3dPayload, TopologyCluster, TopologyLink, TopologyNode,
};
use crate::link_graph::LinkGraphDirection;

use super::shared::{layout_scalar, preferred_label, topology_color, topology_position};

/// Gets 3D topology.
///
/// # Errors
///
/// Returns an error when the graph index cannot be loaded.
pub async fn topology_3d(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<Topology3dPayload>, StudioApiError> {
    let index = state.link_graph_index().await?;
    let docs = index.toc(usize::MAX);
    let total = docs.len();

    let mut nodes = Vec::with_capacity(total);
    let mut cluster_members = BTreeMap::<String, Vec<[f32; 3]>>::new();
    for (position_index, doc) in docs.iter().enumerate() {
        let display_path = studio_display_path(state.studio.as_ref(), doc.path.as_str());
        let cluster_id = display_path
            .split('/')
            .next()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(ToOwned::to_owned);
        let position = topology_position(position_index, total);

        if let Some(cluster_id) = cluster_id.as_ref() {
            cluster_members
                .entry(cluster_id.clone())
                .or_default()
                .push(position);
        }

        nodes.push(TopologyNode {
            id: display_path.clone(),
            name: preferred_label(doc.title.as_str(), display_path.as_str()),
            node_type: "doc".to_string(),
            position,
            cluster_id,
        });
    }

    let mut seen_links = BTreeSet::<(String, String)>::new();
    let mut links = Vec::new();
    for doc in &docs {
        let from = studio_display_path(state.studio.as_ref(), doc.path.as_str());
        for neighbor in
            index.neighbors(doc.id.as_str(), LinkGraphDirection::Outgoing, 1, usize::MAX)
        {
            let to = studio_display_path(state.studio.as_ref(), neighbor.path.as_str());
            if seen_links.insert((from.clone(), to.clone())) {
                links.push(TopologyLink {
                    from: from.clone(),
                    to,
                    label: None,
                });
            }
        }
    }

    let mut clusters = cluster_members
        .into_iter()
        .enumerate()
        .map(|(index, (cluster_id, positions))| {
            let node_count = positions.len();
            let (sum_x, sum_y, sum_z) = positions.into_iter().fold(
                (0.0_f32, 0.0_f32, 0.0_f32),
                |(acc_x, acc_y, acc_z), [x, y, z]| (acc_x + x, acc_y + y, acc_z + z),
            );
            let scale = layout_scalar(node_count.max(1));
            TopologyCluster {
                id: cluster_id.clone(),
                name: cluster_id,
                centroid: [sum_x / scale, sum_y / scale, sum_z / scale],
                node_count,
                color: topology_color(index).to_string(),
            }
        })
        .collect::<Vec<_>>();
    clusters.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(Json(Topology3dPayload {
        nodes,
        links,
        clusters,
    }))
}
