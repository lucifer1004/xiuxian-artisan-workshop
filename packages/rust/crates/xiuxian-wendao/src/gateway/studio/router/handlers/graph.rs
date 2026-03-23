//! Graph intelligence and visualization endpoints for Studio API.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use serde::Deserialize;

use crate::gateway::studio::pathing::{normalize_path_like, studio_display_path};
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{
    GraphLink, GraphNeighborsResponse, GraphNode, NodeNeighbors, Topology3dPayload,
    TopologyCluster, TopologyLink, TopologyNode,
};
use crate::gateway::studio::vfs::resolve_navigation_target;
use crate::link_graph::{LinkGraphDirection, LinkGraphIndex};

const DEFAULT_GRAPH_HOPS: usize = 2;
const DEFAULT_GRAPH_LIMIT: usize = 50;
const MAX_GRAPH_HOPS: usize = 8;
const MAX_GRAPH_LIMIT: usize = 300;
const LEGACY_NEIGHBOR_LIMIT: usize = 200;

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

fn parse_direction(direction: Option<&str>) -> LinkGraphDirection {
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

fn normalize_hops(hops: Option<usize>) -> usize {
    hops.unwrap_or(DEFAULT_GRAPH_HOPS).clamp(1, MAX_GRAPH_HOPS)
}

fn normalize_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_GRAPH_LIMIT)
        .clamp(1, MAX_GRAPH_LIMIT)
}

fn preferred_label(title: &str, fallback_path: &str) -> String {
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

fn resolve_graph_node_id(
    state: &GatewayState,
    index: &LinkGraphIndex,
    node_id: &str,
) -> Option<String> {
    resolve_graph_node_id_by_display_path(state, index, node_id)
}

fn graph_node(
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

fn sorted_unique_paths(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut items = values
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    items.sort();
    items
}

fn topology_position(index: usize, total: usize) -> [f32; 3] {
    if total == 0 {
        return [0.0, 0.0, 0.0];
    }

    let angle = std::f32::consts::TAU * (index as f32) / (total as f32);
    let radius = 14.0 + (index % 7) as f32 * 2.5;
    let depth = (index % 9) as f32 - 4.0;
    [radius * angle.cos(), radius * angle.sin(), depth]
}

fn topology_color(index: usize) -> &'static str {
    const PALETTE: [&str; 8] = [
        "#9ece6a", "#73daca", "#7aa2f7", "#f7768e", "#e0af68", "#bb9af7", "#7dcfff", "#c0caf5",
    ];
    PALETTE[index % PALETTE.len()]
}

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
    let Some(resolved_node_id) =
        resolve_graph_node_id(state.as_ref(), index.as_ref(), node_id.as_str())
    else {
        return Err(StudioApiError::not_found(format!(
            "graph node `{node_id}` was not found"
        )));
    };
    let Some(center_metadata) = index.metadata(resolved_node_id.as_str()) else {
        return Err(StudioApiError::not_found(format!(
            "graph node `{node_id}` was not found"
        )));
    };

    let direction = parse_direction(query.direction.as_deref());
    let hops = normalize_hops(query.hops);
    let limit = normalize_limit(query.limit);

    let neighbors = index.neighbors(resolved_node_id.as_str(), direction, hops, limit);
    let mut nodes = Vec::<GraphNode>::new();
    let mut seen_ids = HashSet::<String>::new();
    let mut internal_paths_by_display_id = HashMap::<String, String>::new();

    let center_node = graph_node(
        state.as_ref(),
        center_metadata.path.as_str(),
        center_metadata.title.as_str(),
        true,
        0,
    );
    internal_paths_by_display_id.insert(
        center_node.id.clone(),
        center_metadata.path.as_str().to_string(),
    );
    seen_ids.insert(center_node.id.clone());
    nodes.push(center_node.clone());

    for neighbor in neighbors {
        let node = graph_node(
            state.as_ref(),
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

    let included_ids = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let mut links = Vec::<GraphLink>::new();
    let mut seen_links = HashSet::<(String, String)>::new();

    for source in &nodes {
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
            let scale = node_count.max(1) as f32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::test_support::assert_studio_json_snapshot;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
    use axum::http::StatusCode;
    use serde_json::json;
    use tempfile::TempDir;

    struct Fixture {
        state: Arc<GatewayState>,
        _temp_dir: TempDir,
    }

    fn build_fixture_with_projects(
        docs: &[(&str, &str)],
        projects: Vec<UiProjectConfig>,
    ) -> Fixture {
        let temp_dir =
            tempfile::tempdir().unwrap_or_else(|error| panic!("create tempdir: {error}"));
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

    fn build_fixture(docs: &[(&str, &str)]) -> Fixture {
        build_fixture_with_projects(
            docs,
            vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec![".".to_string()],
            }],
        )
    }

    #[tokio::test]
    async fn graph_neighbors_returns_center_nodes_and_links() {
        let fixture = build_fixture(&[
            ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
            ("beta.md", "# Beta\n\nBody.\n"),
        ]);

        let response = graph_neighbors(
            State(Arc::clone(&fixture.state)),
            AxumPath("kernel/alpha.md".to_string()),
            Query(GraphNeighborsQuery {
                direction: Some("both".to_string()),
                hops: Some(2),
                limit: Some(20),
            }),
        )
        .await
        .unwrap_or_else(|error| panic!("graph neighbors should succeed: {error:?}"))
        .0;

        assert_eq!(response.center.id, "kernel/alpha.md");
        assert!(
            response
                .nodes
                .iter()
                .any(|node| node.id == "kernel/alpha.md")
        );
        assert!(
            response
                .nodes
                .iter()
                .any(|node| node.id == "kernel/beta.md")
        );
        assert!(
            response.links.iter().any(|link| {
                link.source == "kernel/alpha.md" && link.target == "kernel/beta.md"
            })
        );
        assert!(response.total_nodes >= 2);
        assert!(response.total_links >= 1);
    }

    #[tokio::test]
    async fn graph_neighbors_resolves_relative_markdown_links_from_index_pages() {
        let fixture = build_fixture(&[
            (
                "docs/index.md",
                concat!(
                    "# Documentation Index\n\n",
                    "This file is the top-level entry for major documentation tracks.\n\n",
                    "## Testing\n\n",
                    "- [Testing Documentation](testing/README.md)\n",
                    "- [Skills Tools Benchmark CI Gate](testing/skills-tools-benchmark-ci.md)\n",
                ),
            ),
            (
                "docs/testing/README.md",
                "# Testing Documentation\n\nBody.\n",
            ),
            (
                "docs/testing/skills-tools-benchmark-ci.md",
                "# Skills Tools Benchmark CI Gate\n\nBody.\n",
            ),
        ]);

        let response = graph_neighbors(
            State(Arc::clone(&fixture.state)),
            AxumPath("kernel/docs/index.md".to_string()),
            Query(GraphNeighborsQuery {
                direction: Some("both".to_string()),
                hops: Some(1),
                limit: Some(20),
            }),
        )
        .await
        .unwrap_or_else(|error| {
            panic!("graph neighbors should resolve relative markdown links: {error:?}")
        })
        .0;

        assert!(
            response.total_nodes >= 3,
            "expected docs/index.md to surface related documentation nodes, got {}",
            response.total_nodes
        );
        assert!(
            response.total_links >= 2,
            "expected docs/index.md to surface outbound graph edges, got {}",
            response.total_links
        );
        assert!(
            response
                .nodes
                .iter()
                .any(|node| node.path.contains("testing/README.md")),
            "expected testing/README.md to be present in graph neighbors"
        );
        assert!(
            response
                .nodes
                .iter()
                .any(|node| node.path.contains("testing/skills-tools-benchmark-ci.md")),
            "expected skills-tools-benchmark-ci.md to be present in graph neighbors"
        );
        assert!(
            response
                .links
                .iter()
                .any(|link| link.target.contains("testing/README.md")),
            "expected graph links to point at relative markdown targets"
        );
        assert!(
            response
                .links
                .iter()
                .any(|link| link.target.contains("testing/skills-tools-benchmark-ci.md")),
            "expected graph links to point at relative markdown targets"
        );

        assert_studio_json_snapshot(
            "graph_neighbors_index_page_links_payload",
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
            }),
        );
    }

    #[tokio::test]
    async fn node_neighbors_returns_legacy_payload_shape() {
        let fixture = build_fixture(&[
            ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
            ("beta.md", "# Beta\n\nBody.\n"),
        ]);

        let response = node_neighbors(
            State(Arc::clone(&fixture.state)),
            AxumPath("kernel/alpha.md".to_string()),
        )
        .await
        .unwrap_or_else(|error| panic!("legacy node neighbors should succeed: {error:?}"))
        .0;

        assert_eq!(response.node_id, "kernel/alpha.md");
        assert!(response.outgoing.contains(&"kernel/beta.md".to_string()));
        assert_eq!(response.node_type, "doc");
    }

    #[tokio::test]
    async fn graph_neighbors_returns_not_found_for_missing_node() {
        let fixture = build_fixture(&[("alpha.md", "# Alpha\n\nBody.\n")]);

        let Err(error) = graph_neighbors(
            State(Arc::clone(&fixture.state)),
            AxumPath("missing.md".to_string()),
            Query(GraphNeighborsQuery {
                direction: None,
                hops: None,
                limit: None,
            }),
        )
        .await
        else {
            panic!("missing graph node should fail");
        };

        assert_eq!(error.status(), StatusCode::NOT_FOUND);
        assert_eq!(error.code(), "NOT_FOUND");
    }

    #[tokio::test]
    async fn graph_neighbors_resolves_project_prefixed_display_paths() {
        let fixture = build_fixture(&[
            ("docs/alpha.md", "# Alpha\n\nSee [[beta]].\n"),
            ("docs/beta.md", "# Beta\n\nBody.\n"),
        ]);

        let response = graph_neighbors(
            State(Arc::clone(&fixture.state)),
            AxumPath("kernel/docs/alpha.md".to_string()),
            Query(GraphNeighborsQuery {
                direction: Some("both".to_string()),
                hops: Some(1),
                limit: Some(20),
            }),
        )
        .await
        .unwrap_or_else(|error| panic!("display-path graph neighbors should succeed: {error:?}"))
        .0;

        assert_eq!(response.center.id, "kernel/docs/alpha.md");
        assert!(
            response
                .nodes
                .iter()
                .any(|node| node.id == "kernel/docs/beta.md")
        );
    }

    #[tokio::test]
    async fn graph_neighbors_prefers_exact_display_path_for_project_scoped_index_pages() {
        let fixture = build_fixture_with_projects(
            &[
                (
                    "frontend/docs/index.md",
                    concat!(
                        "---\n",
                        "title: Qianji Studio DocOS Kernel: Map of Content\n",
                        "---\n\n",
                        "# Qianji Studio DocOS Kernel: Map of Content\n\n",
                        "- [[chapter]]\n",
                    ),
                ),
                ("frontend/docs/chapter.md", "# Kernel Chapter\n\nBody.\n"),
                (
                    "docs/index.md",
                    concat!(
                        "---\n",
                        "title: Documentation Index\n",
                        "---\n\n",
                        "# Documentation Index\n\n",
                        "Body.\n",
                    ),
                ),
            ],
            vec![
                UiProjectConfig {
                    name: "kernel".to_string(),
                    root: "frontend".to_string(),
                    dirs: vec!["docs".to_string()],
                },
                UiProjectConfig {
                    name: "main".to_string(),
                    root: ".".to_string(),
                    dirs: vec!["docs".to_string()],
                },
            ],
        );

        let response = graph_neighbors(
            State(Arc::clone(&fixture.state)),
            AxumPath("kernel/docs/index.md".to_string()),
            Query(GraphNeighborsQuery {
                direction: Some("both".to_string()),
                hops: Some(1),
                limit: Some(20),
            }),
        )
        .await
        .unwrap_or_else(|error| panic!("project-scoped graph neighbors should succeed: {error:?}"))
        .0;

        assert_eq!(
            response.center.label,
            "Qianji Studio DocOS Kernel: Map of Content"
        );
        assert!(
            response
                .nodes
                .iter()
                .any(|node| node.id == "kernel/docs/chapter.md"),
            "expected kernel docs chapter to be present in graph neighbors"
        );
        assert!(
            response
                .nodes
                .iter()
                .all(|node| !node.id.starts_with("main/docs/") || node.id == "main/docs/index.md"),
            "expected project-scoped lookup to stay on the kernel document"
        );

        assert_studio_json_snapshot(
            "graph_neighbors_project_scoped_display_path_payload",
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
            }),
        );
    }

    #[tokio::test]
    async fn topology_3d_returns_non_empty_global_graph_payload() {
        let fixture = build_fixture(&[
            ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
            ("beta.md", "# Beta\n\nBody.\n"),
        ]);

        let response = topology_3d(State(Arc::clone(&fixture.state)))
            .await
            .unwrap_or_else(|error| panic!("topology request should succeed: {error:?}"))
            .0;

        assert_eq!(response.nodes.len(), 2);
        assert_eq!(response.links.len(), 1);
        assert!(!response.clusters.is_empty());
        assert!(response.nodes.iter().all(|node| node.position.len() == 3));
    }
}
