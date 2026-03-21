use crate::gateway::studio::types::{
    AnalysisEdge, AnalysisEdgeKind, AnalysisNode, AnalysisNodeKind, MermaidProjection,
    MermaidViewKind,
};

pub(crate) fn build_mermaid_projections(
    nodes: &[AnalysisNode],
    edges: &[AnalysisEdge],
) -> Vec<MermaidProjection> {
    vec![
        build_outline_projection(nodes, edges),
        build_task_projection(nodes, edges),
    ]
}

fn build_outline_projection(nodes: &[AnalysisNode], edges: &[AnalysisEdge]) -> MermaidProjection {
    let mut source = String::from("graph TD\n");
    let mut node_count = 0;
    let mut edge_count = 0;

    for node in nodes {
        if matches!(node.kind, AnalysisNodeKind::Section) {
            source.push_str(&format!("  {}[\"{}\"]\n", escape_id(&node.id), node.label));
            node_count += 1;
        }
    }

    for edge in edges {
        if matches!(
            edge.kind,
            AnalysisEdgeKind::Contains | AnalysisEdgeKind::Parent
        ) {
            let s_id = escape_id(&edge.source_id);
            let t_id = escape_id(&edge.target_id);
            // Rough check if nodes are in this projection
            source.push_str(&format!("  {} --> {}\n", s_id, t_id));
            edge_count += 1;
        }
    }

    MermaidProjection {
        kind: MermaidViewKind::Outline,
        source,
        node_count,
        edge_count,
    }
}

fn build_task_projection(nodes: &[AnalysisNode], edges: &[AnalysisEdge]) -> MermaidProjection {
    let mut source = String::from("graph LR\n");
    let mut node_count = 0;
    let mut edge_count = 0;

    for node in nodes {
        if matches!(node.kind, AnalysisNodeKind::Task) {
            source.push_str(&format!("  {}[\"{}\"]\n", escape_id(&node.id), node.label));
            node_count += 1;
        }
    }

    for edge in edges {
        if matches!(edge.kind, AnalysisEdgeKind::NextStep) {
            source.push_str(&format!(
                "  {} --> {}\n",
                escape_id(&edge.source_id),
                escape_id(&edge.target_id)
            ));
            edge_count += 1;
        }
    }

    MermaidProjection {
        kind: MermaidViewKind::Tasks,
        source,
        node_count,
        edge_count,
    }
}

fn escape_id(id: &str) -> String {
    id.replace(':', "_").replace('.', "_").replace('-', "_")
}
