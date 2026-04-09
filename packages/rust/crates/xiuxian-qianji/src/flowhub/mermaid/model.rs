/// One parsed Mermaid node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MermaidNode {
    /// Stable Mermaid identifier.
    pub(crate) id: String,
    /// Visible label if one was declared; otherwise the id.
    pub(crate) label: String,
    /// Whether the node resolved to a known Flowhub module.
    pub(crate) kind: MermaidNodeKind,
}

/// Classification of one Mermaid node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MermaidNodeKind {
    /// Node label matches a known Flowhub module name.
    Module,
    /// Node is a scenario-local or guard node.
    Scenario,
}

/// One parsed Mermaid edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MermaidEdge {
    /// Source node id.
    pub(crate) from: String,
    /// Destination node id.
    pub(crate) to: String,
}

/// Minimal parsed Mermaid flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MermaidFlowchart {
    /// Mermaid graph identity, derived from the owning Mermaid filename stem.
    pub(crate) merimind_graph_name: String,
    /// Declared Mermaid direction, for example `LR`.
    pub(crate) direction: String,
    /// Parsed nodes in stable declaration order.
    pub(crate) nodes: Vec<MermaidNode>,
    /// Parsed edges in declaration order.
    pub(crate) edges: Vec<MermaidEdge>,
}
