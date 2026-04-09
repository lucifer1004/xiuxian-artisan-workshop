use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{MermaidFlowchart, MermaidNodeKind};

const ALLOWED_SCENARIO_GRAPH_NODE_LABELS: &[&str] = &[
    "Codex write bounded surface",
    "surface check",
    "flowchart alignment",
    "boundary check",
    "drift check",
    "boundary and drift check",
    "status legality",
    "domain validators",
    "done gate",
    "diagnostics",
];

pub(crate) fn validate_mermaid_flowchart(
    flowchart: &MermaidFlowchart,
    registered_module_names: &[String],
) -> Result<(), String> {
    if flowchart.edges.is_empty() {
        return Err("scenario-case graph must declare at least one edge".to_string());
    }

    let nodes_by_id = flowchart
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    for edge in &flowchart.edges {
        if !nodes_by_id.contains_key(edge.from.as_str())
            || !nodes_by_id.contains_key(edge.to.as_str())
        {
            return Err(format!(
                "scenario-case graph contains an edge `{}` -> `{}` whose endpoint is not declared as a Mermaid node",
                edge.from, edge.to
            ));
        }
    }

    let undeclared_graph_node_labels = flowchart
        .nodes
        .iter()
        .filter(|node| node.kind != MermaidNodeKind::Module)
        .filter(|node| !ALLOWED_SCENARIO_GRAPH_NODE_LABELS.contains(&node.label.as_str()))
        .map(|node| node.label.as_str())
        .collect::<Vec<_>>();
    if !undeclared_graph_node_labels.is_empty() {
        return Err(format!(
            "scenario-case graph `{}` contains undeclared graph nodes: {}",
            flowchart.merimind_graph_name,
            undeclared_graph_node_labels.join(", ")
        ));
    }

    let module_nodes = flowchart
        .nodes
        .iter()
        .filter(|node| node.kind == MermaidNodeKind::Module)
        .collect::<Vec<_>>();
    let declared_module_labels = module_nodes
        .iter()
        .map(|node| node.label.as_str())
        .collect::<BTreeSet<_>>();
    let missing_registered_module_labels = registered_module_names
        .iter()
        .filter(|module_name| !declared_module_labels.contains(module_name.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !missing_registered_module_labels.is_empty() {
        return Err(format!(
            "scenario-case graph `{}` is missing registered Flowhub module nodes: {}",
            flowchart.merimind_graph_name,
            missing_registered_module_labels.join(", ")
        ));
    }

    if module_nodes.len() < 2 {
        return Err(
            "scenario-case graph must expose at least two Flowhub module nodes".to_string(),
        );
    }

    let module_node_ids = module_nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let module_edges = flowchart
        .edges
        .iter()
        .filter(|edge| {
            module_node_ids.contains(edge.from.as_str())
                && module_node_ids.contains(edge.to.as_str())
        })
        .collect::<Vec<_>>();
    if module_edges.is_empty() {
        return Err(
            "scenario-case graph must declare at least one edge between Flowhub module nodes"
                .to_string(),
        );
    }

    let mut module_nodes_with_backbone_edge = BTreeSet::new();
    for edge in &module_edges {
        module_nodes_with_backbone_edge.insert(edge.from.as_str());
        module_nodes_with_backbone_edge.insert(edge.to.as_str());
    }
    let isolated_module_labels = module_nodes
        .iter()
        .filter(|node| !module_nodes_with_backbone_edge.contains(node.id.as_str()))
        .map(|node| node.label.as_str())
        .collect::<Vec<_>>();
    if !isolated_module_labels.is_empty() {
        return Err(format!(
            "scenario-case graph contains Flowhub module nodes without a module-backbone edge: {}",
            isolated_module_labels.join(", ")
        ));
    }

    let mut adjacency = BTreeMap::<&str, BTreeSet<&str>>::new();
    for edge in &module_edges {
        adjacency
            .entry(edge.from.as_str())
            .or_default()
            .insert(edge.to.as_str());
        adjacency
            .entry(edge.to.as_str())
            .or_default()
            .insert(edge.from.as_str());
    }

    let start = module_nodes[0].id.as_str();
    let mut queue = VecDeque::from([start]);
    let mut visited = BTreeSet::new();
    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id) {
            continue;
        }
        for neighbor in adjacency.get(node_id).into_iter().flatten() {
            if !visited.contains(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }

    let disconnected_module_labels = module_nodes
        .iter()
        .filter(|node| !visited.contains(node.id.as_str()))
        .map(|node| node.label.as_str())
        .collect::<Vec<_>>();
    if !disconnected_module_labels.is_empty() {
        return Err(format!(
            "scenario-case graph contains disconnected Flowhub module backbone nodes: {}",
            disconnected_module_labels.join(", ")
        ));
    }

    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit/flowhub/mermaid/validate.rs"]
mod tests;
