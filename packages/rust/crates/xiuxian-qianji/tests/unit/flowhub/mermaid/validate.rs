use super::validate_mermaid_flowchart;
use crate::flowhub::mermaid::parse_mermaid_flowchart;

#[test]
fn rejects_flowchart_without_module_nodes() {
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"diagnostics\"] --> B[\"done gate\"]\n",
        "codex-plan",
        &[],
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    let error = validate_mermaid_flowchart(&parsed, &[])
        .err()
        .unwrap_or_else(|| panic!("missing module nodes should fail"));
    assert!(error.contains("at least two Flowhub module nodes"));
}

#[test]
fn rejects_flowchart_without_module_backbone_edge() {
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> X[\"diagnostics\"]\n  B[\"plan\"] --> Y[\"done gate\"]\n",
        "codex-plan",
        &["coding".to_string(), "plan".to_string()],
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    let error = validate_mermaid_flowchart(&parsed, &["coding".to_string(), "plan".to_string()])
        .err()
        .unwrap_or_else(|| panic!("missing module backbone edge should fail"));
    assert!(error.contains("at least one edge between Flowhub module nodes"));
}

#[test]
fn rejects_disconnected_module_backbone() {
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  C[\"blueprint\"] --> D[\"plan\"]\n",
        "codex-plan",
        &[
            "coding".to_string(),
            "rust".to_string(),
            "blueprint".to_string(),
            "plan".to_string(),
        ],
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    let error = validate_mermaid_flowchart(
        &parsed,
        &[
            "coding".to_string(),
            "rust".to_string(),
            "blueprint".to_string(),
            "plan".to_string(),
        ],
    )
    .err()
    .unwrap_or_else(|| panic!("disconnected module backbone should fail"));
    assert!(error.contains("disconnected Flowhub module backbone nodes"));
    assert!(error.contains("blueprint") || error.contains("plan"));
}

#[test]
fn rejects_flowchart_missing_registered_module_nodes() {
    let registered_module_names = vec![
        "coding".to_string(),
        "rust".to_string(),
        "blueprint".to_string(),
        "plan".to_string(),
    ];
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  B --> C[\"diagnostics\"]\n",
        "codex-plan",
        &registered_module_names,
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    let error = validate_mermaid_flowchart(&parsed, &registered_module_names)
        .err()
        .unwrap_or_else(|| panic!("missing registered module nodes should fail"));
    assert!(error.contains("codex-plan"));
    assert!(error.contains("missing registered Flowhub module nodes"));
    assert!(error.contains("blueprint"));
    assert!(error.contains("plan"));
}

#[test]
fn rejects_flowchart_with_undeclared_graph_nodes() {
    let registered_module_names = vec![
        "coding".to_string(),
        "rust".to_string(),
        "blueprint".to_string(),
        "plan".to_string(),
    ];
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  B --> C[\"style\"]\n  C --> D[\"blueprint\"]\n  D --> E[\"plan\"]\n",
        "codex-plan",
        &registered_module_names,
    )
    .unwrap_or_else(|error| panic!("flowchart should parse: {error}"));

    let error = validate_mermaid_flowchart(&parsed, &registered_module_names)
        .err()
        .unwrap_or_else(|| panic!("undeclared graph nodes should fail"));
    assert!(error.contains("codex-plan"));
    assert!(error.contains("undeclared graph nodes"));
    assert!(error.contains("style"));
}
