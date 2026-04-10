//! Contract tests for Flowhub root/module discovery, show, and check.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use xiuxian_config_core::resolve_project_root;
use xiuxian_qianji::{
    FlowhubGraphNodeKind, FlowhubModuleKind, FlowhubScenarioCaseSummary, FlowhubShow,
    check_flowhub, classify_flowhub_dir, render_flowhub_check_markdown, render_flowhub_graph_show,
    render_flowhub_show, show_flowhub, show_flowhub_graph,
};

fn repo_root() -> PathBuf {
    resolve_project_root()
        .unwrap_or_else(|| panic!("workspace root should resolve from PRJ_ROOT or git ancestry"))
}

fn flowhub_root() -> PathBuf {
    repo_root().join("qianji-flowhub")
}

fn assert_common_diagnostic_shape(rendered: &str) {
    assert!(rendered.contains("# Validation Failed"));
    assert!(rendered.contains("Location:"));
    assert!(rendered.contains("Problem:"));
    assert!(rendered.contains("Why it blocks:"));
    assert!(rendered.contains("Fix:"));
}

fn assert_common_show_shape(rendered: &str) {
    assert!(rendered.starts_with("# "));
    assert!(rendered.contains("Location:"));
    assert!(rendered.contains("\n## "));
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("should create {}: {error}", parent.display()));
    }
    fs::write(path, content)
        .unwrap_or_else(|error| panic!("should write {}: {error}", path.display()));
}

fn create_invalid_flowhub(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let module_dir = root.join("broken-module");
    fs::create_dir_all(&module_dir).unwrap_or_else(|error| {
        panic!("should create module dir {}: {error}", module_dir.display())
    });
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["broken-module"]
required = ["*/qianji.toml"]
"#,
    );
    write_file(
        &module_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "broken-module"
tags = ["planning", "broken"]

[exports]
entry = "task.start"
ready = "task.ready"

[contract]
register = ["missing-child"]
required = ["*/qianji.toml"]
"#,
    );
    root
}

fn create_missing_root_contract_flowhub(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let module_dir = root.join("coding");
    fs::create_dir_all(&module_dir).unwrap_or_else(|error| {
        panic!("should create module dir {}: {error}", module_dir.display())
    });
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "broken-flowhub"
"#,
    );
    write_file(
        &module_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "coding"
tags = ["planning", "coding"]

[exports]
entry = "task.coding-start"
ready = "task.coding-ready"
"#,
    );
    root
}

fn create_leaf_with_unregistered_child_dir_flowhub(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let rust_dir = root.join("rust");
    let stray_dir = rust_dir.join("style");
    fs::create_dir_all(&stray_dir)
        .unwrap_or_else(|error| panic!("should create stray dir {}: {error}", stray_dir.display()));
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["rust"]
required = ["*/qianji.toml"]
"#,
    );
    write_file(
        &rust_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "rust"
tags = ["planning", "coding", "rust"]

[exports]
entry = "task.rust-start"
ready = "task.rust-ready"
"#,
    );
    root
}

fn create_flowhub_with_unregistered_top_level_dir(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let rust_dir = root.join("rust");
    let stray_dir = root.join("scratch");
    fs::create_dir_all(&rust_dir)
        .unwrap_or_else(|error| panic!("should create rust dir {}: {error}", rust_dir.display()));
    fs::create_dir_all(&stray_dir).unwrap_or_else(|error| {
        panic!(
            "should create stray top-level dir {}: {error}",
            stray_dir.display()
        )
    });
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["rust"]
required = ["*/qianji.toml"]
"#,
    );
    write_file(
        &rust_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "rust"
tags = ["planning", "coding", "rust"]

[exports]
entry = "task.rust-start"
ready = "task.rust-ready"
"#,
    );
    root
}

fn create_flowhub_with_invalid_mermaid_case(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let plan_dir = root.join("plan");
    fs::create_dir_all(&plan_dir)
        .unwrap_or_else(|error| panic!("should create plan dir {}: {error}", plan_dir.display()));
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["plan"]
required = ["*/qianji.toml"]
"#,
    );
    write_file(
        &plan_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "plan"
tags = ["planning", "plan"]

[exports]
entry = "task.plan-start"
ready = "task.plan-ready"

[contract]
required = ["codex-plan.mmd"]
"#,
    );
    write_file(
        &plan_dir.join("codex-plan.mmd"),
        r#"
flowchart LR
  A["diagnostics"]
"#,
    );
    root
}

fn create_flowhub_with_disconnected_mermaid_case(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let plan_dir = root.join("plan");
    fs::create_dir_all(&plan_dir)
        .unwrap_or_else(|error| panic!("should create plan dir {}: {error}", plan_dir.display()));
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["coding", "rust", "blueprint", "plan"]
required = ["*/qianji.toml"]
"#,
    );
    for module_name in ["coding", "rust", "blueprint"] {
        let module_dir = root.join(module_name);
        fs::create_dir_all(&module_dir).unwrap_or_else(|error| {
            panic!("should create module dir {}: {error}", module_dir.display())
        });
        write_file(
            &module_dir.join("qianji.toml"),
            &format!(
                r#"
version = 1

[module]
name = "{module_name}"
tags = ["planning", "{module_name}"]

[exports]
entry = "task.{module_name}-start"
ready = "task.{module_name}-ready"
"#
            ),
        );
    }
    write_file(
        &plan_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "plan"
tags = ["planning", "plan"]

[exports]
entry = "task.plan-start"
ready = "task.plan-ready"

[contract]
required = ["codex-plan.mmd"]
"#,
    );
    write_file(
        &plan_dir.join("codex-plan.mmd"),
        r#"
flowchart LR
  A["coding"] --> B["rust"]
  C["blueprint"] --> D["plan"]
"#,
    );
    root
}

fn create_flowhub_with_missing_registered_mermaid_nodes_case(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let plan_dir = root.join("plan");
    fs::create_dir_all(&plan_dir)
        .unwrap_or_else(|error| panic!("should create plan dir {}: {error}", plan_dir.display()));
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["coding", "rust", "blueprint", "plan"]
required = ["*/qianji.toml"]
"#,
    );
    for module_name in ["coding", "rust", "blueprint"] {
        let module_dir = root.join(module_name);
        fs::create_dir_all(&module_dir).unwrap_or_else(|error| {
            panic!("should create module dir {}: {error}", module_dir.display())
        });
        write_file(
            &module_dir.join("qianji.toml"),
            &format!(
                r#"
version = 1

[module]
name = "{module_name}"
tags = ["planning", "{module_name}"]

[exports]
entry = "task.{module_name}-start"
ready = "task.{module_name}-ready"
"#
            ),
        );
    }
    write_file(
        &plan_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "plan"
tags = ["planning", "plan"]

[exports]
entry = "task.plan-start"
ready = "task.plan-ready"

[contract]
required = ["codex-plan.mmd"]
"#,
    );
    write_file(
        &plan_dir.join("codex-plan.mmd"),
        r#"
flowchart LR
  A["coding"] --> B["rust"]
  B --> C["diagnostics"]
"#,
    );
    root
}

fn create_flowhub_with_undeclared_mermaid_nodes_case(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let plan_dir = root.join("plan");
    fs::create_dir_all(&plan_dir)
        .unwrap_or_else(|error| panic!("should create plan dir {}: {error}", plan_dir.display()));
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["coding", "rust", "blueprint", "plan"]
required = ["*/qianji.toml"]
"#,
    );
    for module_name in ["coding", "rust", "blueprint"] {
        let module_dir = root.join(module_name);
        fs::create_dir_all(&module_dir).unwrap_or_else(|error| {
            panic!("should create module dir {}: {error}", module_dir.display())
        });
        write_file(
            &module_dir.join("qianji.toml"),
            &format!(
                r#"
version = 1

[module]
name = "{module_name}"
tags = ["planning", "{module_name}"]

[exports]
entry = "task.{module_name}-start"
ready = "task.{module_name}-ready"
"#
            ),
        );
    }
    write_file(
        &plan_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "plan"
tags = ["planning", "plan"]

[exports]
entry = "task.plan-start"
ready = "task.plan-ready"

[contract]
required = ["codex-plan.mmd"]
"#,
    );
    write_file(
        &plan_dir.join("codex-plan.mmd"),
        r#"
flowchart LR
  A["coding"] --> B["rust"]
  B --> C["style"]
  C --> D["blueprint"]
  D --> E["plan"]
"#,
    );
    root
}

fn create_flowhub_with_mermaid_presentation_directives_case(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().join("flowhub");
    let plan_dir = root.join("plan");
    fs::create_dir_all(&plan_dir)
        .unwrap_or_else(|error| panic!("should create plan dir {}: {error}", plan_dir.display()));
    write_file(
        &root.join("qianji.toml"),
        r#"
version = 1

[flowhub]
name = "test-flowhub"

[contract]
register = ["coding", "rust", "blueprint", "plan"]
required = ["*/qianji.toml"]
"#,
    );
    for module_name in ["coding", "rust", "blueprint"] {
        let module_dir = root.join(module_name);
        fs::create_dir_all(&module_dir).unwrap_or_else(|error| {
            panic!("should create module dir {}: {error}", module_dir.display())
        });
        write_file(
            &module_dir.join("qianji.toml"),
            &format!(
                r#"
version = 1

[module]
name = "{module_name}"
tags = ["planning", "{module_name}"]

[exports]
entry = "task.{module_name}-start"
ready = "task.{module_name}-ready"
"#
            ),
        );
    }
    write_file(
        &plan_dir.join("qianji.toml"),
        r#"
version = 1

[module]
name = "plan"
tags = ["planning", "plan"]

[exports]
entry = "task.plan-start"
ready = "task.plan-ready"

[contract]
required = ["codex-plan.mmd"]
"#,
    );
    write_file(
        &plan_dir.join("codex-plan.mmd"),
        r#"
flowchart LR
  A["coding"] --> B["rust"]
  B --> C["blueprint"]
  C --> D["plan"]

  D --> E["Codex write bounded surface"]
  E --> F["surface check"]
  F --> G["flowchart alignment"]
  G --> H["boundary and drift check"]
  H --> I["domain validators"]
  I --> J["done gate"]

  F -- fail --> R["diagnostics"]
  G -- fail --> R
  H -- fail --> R
  I -- fail --> R
  R --> E

  classDef highlight fill:#f9f,stroke:#333,stroke-width:2px;
  class A,B highlight;
  style C fill:#e0f7fa,stroke:#006064;
  click G "https://example.com/flowchart-alignment" "flowchart alignment docs"
"#,
    );
    root
}

#[test]
fn classify_flowhub_dir_detects_real_root_and_module() {
    assert_eq!(
        classify_flowhub_dir(flowhub_root())
            .unwrap_or_else(|error| panic!("root should classify: {error}")),
        Some(xiuxian_qianji::FlowhubDirKind::Root)
    );
    assert_eq!(
        classify_flowhub_dir(flowhub_root().join("rust"))
            .unwrap_or_else(|error| panic!("module should classify: {error}")),
        Some(xiuxian_qianji::FlowhubDirKind::Module)
    );
}

#[test]
fn show_flowhub_summarizes_real_root() {
    let show = show_flowhub(flowhub_root())
        .unwrap_or_else(|error| panic!("real Flowhub root should show: {error}"));

    let FlowhubShow::Root(show) = show else {
        panic!("expected Flowhub root summary");
    };
    assert_eq!(show.modules.len(), 4);
    assert!(
        show.modules
            .iter()
            .any(|module| module.module_ref == "rust")
    );
    assert!(
        show.modules
            .iter()
            .any(|module| module.module_ref == "blueprint")
    );

    let rendered = render_flowhub_show(&FlowhubShow::Root(show));
    assert_common_show_shape(&rendered);
    assert!(rendered.contains("# Flowhub"));
    assert!(rendered.contains("## rust"));
}

#[test]
fn show_flowhub_summarizes_real_leaf_module() {
    let show = show_flowhub(flowhub_root().join("rust"))
        .unwrap_or_else(|error| panic!("real Flowhub module should show: {error}"));

    let FlowhubShow::Module(show) = show else {
        panic!("expected Flowhub module summary");
    };
    assert_eq!(show.summary.module_ref, "rust");
    assert_eq!(show.summary.kind, FlowhubModuleKind::Leaf);
    assert!(show.summary.child_modules.is_empty());

    let rendered = render_flowhub_show(&FlowhubShow::Module(show));
    assert_common_show_shape(&rendered);
    assert!(rendered.contains("# Flowhub Module"));
    assert!(rendered.contains("Module: rust"));
    assert!(rendered.contains("## Contract"));
    assert!(rendered.contains("Registered children: 0"));
}

#[test]
fn show_flowhub_keeps_required_only_plan_node_as_leaf() {
    let show = show_flowhub(flowhub_root().join("plan"))
        .unwrap_or_else(|error| panic!("plan node should show: {error}"));

    let FlowhubShow::Module(show) = show else {
        panic!("expected Flowhub module summary");
    };
    assert_eq!(show.summary.module_ref, "plan");
    assert_eq!(show.summary.kind, FlowhubModuleKind::Leaf);
    assert_eq!(show.registered_child_count, 0);
    assert_eq!(show.required_contract_count, 1);
    assert_eq!(
        show.scenario_cases,
        vec![FlowhubScenarioCaseSummary {
            file_name: "codex-plan.mmd".to_string(),
            merimind_graph_name: "codex-plan".to_string(),
        }]
    );

    let rendered = render_flowhub_show(&FlowhubShow::Module(show));
    assert_common_show_shape(&rendered);
    assert!(rendered.contains("Required contract entries: 1"));
    assert!(rendered.contains("## Scenario Cases"));
    assert!(rendered.contains("Graph name: codex-plan"));
    assert!(rendered.contains("Path: ./plan/codex-plan.mmd"));
}

#[test]
fn show_flowhub_graph_extracts_live_mermaid_nodes_edges_and_exports() {
    let show = show_flowhub_graph(flowhub_root().join("plan/codex-plan.mmd"))
        .unwrap_or_else(|error| panic!("live Mermaid graph should show: {error}"));

    assert_eq!(show.merimind_graph_name, "codex-plan");
    assert_eq!(show.kind, "scenario");
    assert_eq!(show.owning_module_ref, "plan");
    assert_eq!(show.direction, "LR");
    assert!(show.mermaid.contains("flowchart LR"));
    assert!(show.nodes.iter().any(|node| {
        node.label == "coding"
            && node.kind == FlowhubGraphNodeKind::Context
            && node.exports_entry.as_deref() == Some("task.coding-start")
    }));
    assert!(show.nodes.iter().any(|node| {
        node.label == "domain validators"
            && node.kind == FlowhubGraphNodeKind::Validator
            && node.next == vec!["done gate".to_string(), "diagnostics".to_string()]
    }));
    assert!(show.nodes.iter().any(|node| {
        node.label == "plan"
            && node.kind == FlowhubGraphNodeKind::Artifact
            && node.next == vec!["Codex write bounded surface".to_string()]
            && node.exports_ready.as_deref() == Some("task.plan-ready")
    }));
    assert!(
        show.expected_work_surface
            .contains(&"qianji.toml".to_string())
    );
    assert!(show.local_contract_template.contains("[plan]"));
    assert!(show.missing_registered_modules.is_empty());
    assert!(show.unknown_graph_nodes.is_empty());

    let rendered = render_flowhub_graph_show(&show);
    assert!(rendered.starts_with("# Graph"));
    assert!(rendered.contains("Name: codex-plan"));
    assert!(rendered.contains("Kind: scenario"));
    assert!(rendered.contains("## Mermaid"));
    assert!(rendered.contains("```mermaid"));
    assert!(rendered.contains("## Nodes"));
    assert!(rendered.contains("### coding"));
    assert!(rendered.contains("Kind: context"));
    assert!(rendered.contains("### boundary and drift check"));
    assert!(rendered.contains("Kind: guard"));
    assert!(rendered.contains("## Expected work surface"));
    assert!(rendered.contains("## Local qianji.toml template"));
}

#[test]
fn show_flowhub_graph_surfaces_unknown_graph_nodes() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_undeclared_mermaid_nodes_case(&temp_dir);

    let show = show_flowhub_graph(root.join("plan/codex-plan.mmd"))
        .unwrap_or_else(|error| panic!("Mermaid graph with unknown nodes should show: {error}"));

    assert_eq!(show.unknown_graph_nodes, vec!["style".to_string()]);
    assert!(show.nodes.iter().any(|node| {
        node.label == "style"
            && node.kind == FlowhubGraphNodeKind::Unknown
            && node.agent_action
                == "do not rely on this node until the Flowhub graph contract is corrected"
    }));
    let rendered = render_flowhub_graph_show(&show);
    assert!(rendered.contains("### style"));
    assert!(rendered.contains("Kind: unknown"));
    assert!(rendered.contains(
        "Agent action: do not rely on this node until the Flowhub graph contract is corrected"
    ));
}

#[test]
fn show_flowhub_graph_preserves_raw_mermaid_but_ignores_presentation_directives_in_semantics() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_mermaid_presentation_directives_case(&temp_dir);

    let show = show_flowhub_graph(root.join("plan/codex-plan.mmd")).unwrap_or_else(|error| {
        panic!("Mermaid graph with presentation directives should show: {error}")
    });

    assert!(show.mermaid.contains("classDef highlight"));
    assert!(show.mermaid.contains("style C"));
    assert!(show.mermaid.contains("click G"));
    assert!(show.unknown_graph_nodes.is_empty());
    assert!(!show.nodes.iter().any(|node| node.label == "highlight"));
    assert!(
        !show
            .nodes
            .iter()
            .any(|node| node.label.contains("https://"))
    );
    assert!(
        show.nodes
            .iter()
            .any(|node| node.label == "flowchart alignment")
    );

    let rendered = render_flowhub_graph_show(&show);
    assert!(rendered.contains("classDef highlight"));
    assert!(rendered.contains("style C"));
    assert!(rendered.contains("click G"));
    assert!(!rendered.contains("### highlight"));
}

#[test]
fn check_flowhub_accepts_real_root() {
    let report = check_flowhub(flowhub_root())
        .unwrap_or_else(|error| panic!("real Flowhub root should check: {error}"));

    assert!(report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert!(rendered.contains("# Validation Passed"));
}

#[test]
fn check_flowhub_reports_invalid_mermaid_scenario_case() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_invalid_mermaid_case(&temp_dir);

    let report = check_flowhub(&root)
        .unwrap_or_else(|error| panic!("invalid Mermaid case should still report: {error}"));

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Invalid scenario-case graph"));
    assert!(rendered.contains("codex-plan.mmd"));
}

#[test]
fn check_flowhub_reports_disconnected_mermaid_module_backbone() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_disconnected_mermaid_case(&temp_dir);

    let report = check_flowhub(&root)
        .unwrap_or_else(|error| panic!("disconnected Mermaid case should still report: {error}"));

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Invalid scenario-case graph"));
    assert!(rendered.contains("disconnected Flowhub module backbone nodes"));
    assert!(rendered.contains("codex-plan.mmd"));
}

#[test]
fn check_flowhub_reports_missing_registered_module_nodes_in_mermaid_case() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_missing_registered_mermaid_nodes_case(&temp_dir);

    let report = check_flowhub(&root).unwrap_or_else(|error| {
        panic!("Mermaid case missing registered modules should still report: {error}")
    });

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Invalid scenario-case graph"));
    assert!(rendered.contains("missing registered Flowhub module nodes"));
    assert!(rendered.contains("blueprint"));
    assert!(rendered.contains("plan"));
    assert!(rendered.contains("codex-plan.mmd"));
}

#[test]
fn check_flowhub_reports_undeclared_graph_nodes_in_mermaid_case() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_undeclared_mermaid_nodes_case(&temp_dir);

    let report = check_flowhub(&root).unwrap_or_else(|error| {
        panic!("Mermaid case with undeclared graph nodes should still report: {error}")
    });

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Invalid scenario-case graph"));
    assert!(rendered.contains("undeclared graph nodes"));
    assert!(rendered.contains("style"));
    assert!(rendered.contains("codex-plan.mmd"));
}

#[test]
fn check_flowhub_accepts_mermaid_case_with_presentation_directives() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_mermaid_presentation_directives_case(&temp_dir);

    let report = check_flowhub(&root).unwrap_or_else(|error| {
        panic!("Mermaid case with presentation directives should still report: {error}")
    });

    assert!(report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert!(rendered.contains("# Validation Passed"));
}

#[test]
fn check_flowhub_reports_missing_required_module_paths() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_invalid_flowhub(&temp_dir);

    let report = check_flowhub(&root)
        .unwrap_or_else(|error| panic!("invalid Flowhub should still report: {error}"));

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Missing contract path"));
    assert!(rendered.contains("broken-module"));
    assert!(!rendered.contains("## Follow-up Query"));
}

#[test]
fn check_flowhub_blocks_missing_root_contract() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_missing_root_contract_flowhub(&temp_dir);

    let report = check_flowhub(&root)
        .unwrap_or_else(|error| panic!("invalid Flowhub root should still report: {error}"));

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Invalid Flowhub root contract"));
    assert!(rendered.contains("[contract]"));
}

#[test]
fn check_flowhub_blocks_unregistered_child_directory_under_leaf_node() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_leaf_with_unregistered_child_dir_flowhub(&temp_dir);

    let report = check_flowhub(&root)
        .unwrap_or_else(|error| panic!("Flowhub drift should still report: {error}"));

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Unregistered child directory"));
    assert!(rendered.contains("style"));
    assert!(rendered.contains("contract.register"));
}

#[test]
fn check_flowhub_blocks_unregistered_top_level_directory() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let root = create_flowhub_with_unregistered_top_level_dir(&temp_dir);

    let report = check_flowhub(&root)
        .unwrap_or_else(|error| panic!("Flowhub top-level drift should still report: {error}"));

    assert!(!report.is_valid());
    let rendered = render_flowhub_check_markdown(&report);
    assert_common_diagnostic_shape(&rendered);
    assert!(rendered.contains("Unregistered Flowhub module"));
    assert!(rendered.contains("scratch"));
    assert!(rendered.contains("contract.register"));
}

xiuxian_testing::crate_test_policy_harness!();
