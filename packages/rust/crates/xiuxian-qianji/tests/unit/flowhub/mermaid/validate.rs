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
    assert!(error.contains("at least one Flowhub module node"));
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

#[test]
fn accepts_flowchart_with_presentation_directives() {
    let registered_module_names = vec![
        "coding".to_string(),
        "rust".to_string(),
        "blueprint".to_string(),
        "plan".to_string(),
    ];
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"coding\"] --> B[\"rust\"]\n  B --> C[\"blueprint\"]\n  C --> D[\"plan\"]\n  classDef highlight fill:#f9f,stroke:#333,stroke-width:2px;\n  class A,B highlight;\n  style C fill:#e0f7fa,stroke:#006064;\n  click D \"https://example.com/plan\" \"plan docs\"\n",
        "codex-plan",
        &registered_module_names,
    )
    .unwrap_or_else(|error| panic!("presentation directives should not break parse: {error}"));

    validate_mermaid_flowchart(&parsed, &registered_module_names).unwrap_or_else(|error| {
        panic!("presentation directives should not break validation: {error}")
    });
}

#[test]
fn accepts_flowchart_with_exact_http_request_labels() {
    let registered_module_names = vec!["wendao".to_string()];
    let parsed = parse_mermaid_flowchart(
        "flowchart LR\n  A[\"wendao\"] --> B[\"GET /api/docs/search?repo=<repo>&query=<query>&kind=<kind>&limit=<n>\"]\n  B --> C[\"GET /api/docs/page?repo=<repo>&page_id=<page_id>\"]\n  C --> D[\"done gate\"]\n",
        "wendao-search",
        &registered_module_names,
    )
    .unwrap_or_else(|error| panic!("http request labels should parse: {error}"));

    validate_mermaid_flowchart(&parsed, &registered_module_names)
        .unwrap_or_else(|error| panic!("http request labels should validate: {error}"));
}
