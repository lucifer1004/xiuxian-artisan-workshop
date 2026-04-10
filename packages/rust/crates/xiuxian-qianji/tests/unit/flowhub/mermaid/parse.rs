use super::parse_mermaid_flowchart;
use crate::flowhub::mermaid::MermaidNodeKind;

#[test]
fn parses_flowchart_with_module_and_scenario_nodes() {
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  B --> C[\"diagnostics\"]\n",
        "codex-plan",
        &["coding".to_string(), "rust".to_string()],
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    assert_eq!(parsed.merimind_graph_name, "codex-plan");
    assert_eq!(parsed.direction, "LR");
    assert_eq!(parsed.edges.len(), 2);
    assert_eq!(parsed.nodes.len(), 3);
    assert_eq!(parsed.nodes[0].kind, MermaidNodeKind::Module);
    assert_eq!(parsed.nodes[1].kind, MermaidNodeKind::Module);
    assert_eq!(parsed.nodes[2].kind, MermaidNodeKind::Scenario);
}

#[test]
fn accepts_bare_node_references_after_explicit_labels() {
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  B --> C[\"blueprint\"]\n  C --> D[\"plan\"]\n",
        "codex-plan",
        &[
            "coding".to_string(),
            "rust".to_string(),
            "blueprint".to_string(),
            "plan".to_string(),
        ],
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    let rust_node = parsed
        .nodes
        .iter()
        .find(|node| node.id == "B")
        .unwrap_or_else(|| panic!("rust node should exist"));
    assert_eq!(rust_node.label, "rust");
    assert_eq!(rust_node.kind, MermaidNodeKind::Module);
}

#[test]
fn rejects_conflicting_explicit_node_labels() {
    let error = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  A[\"plan\"] --> B\n",
        "codex-plan",
        &["coding".to_string(), "rust".to_string(), "plan".to_string()],
    )
    .err()
    .unwrap_or_else(|| panic!("conflicting labels should fail"));

    assert!(error.to_string().contains("conflicting labels"));
}

#[test]
fn ignores_presentation_directives_when_extracting_first_order_graph_semantics() {
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  B --> C[\"blueprint\"]\n  C --> D[\"plan\"]\n  classDef highlight fill:#f9f,stroke:#333,stroke-width:2px;\n  class A,B highlight;\n  style C fill:#e0f7fa,stroke:#006064;\n  click D \"https://example.com/plan\" \"plan docs\"\n",
        "codex-plan",
        &[
            "coding".to_string(),
            "rust".to_string(),
            "blueprint".to_string(),
            "plan".to_string(),
        ],
    )
    .unwrap_or_else(|error| panic!("presentation directives should not break parse: {error}"));

    assert_eq!(parsed.direction, "LR");
    assert_eq!(parsed.edges.len(), 3);
    assert_eq!(
        parsed
            .nodes
            .iter()
            .map(|node| node.label.as_str())
            .collect::<Vec<_>>(),
        vec!["coding", "rust", "blueprint", "plan"]
    );
    assert!(!parsed.nodes.iter().any(|node| node.label == "highlight"));
    assert!(
        !parsed
            .nodes
            .iter()
            .any(|node| node.label.contains("https://"))
    );
}
