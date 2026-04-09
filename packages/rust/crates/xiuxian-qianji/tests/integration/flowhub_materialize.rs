//! Materialize tests for Flowhub scenario-to-workdir generation.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;
use xiuxian_qianji::{check_workdir, materialize_flowhub_scenario_workdir, show_workdir};

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("should create {}: {error}", parent.display()));
    }
    fs::write(path, content)
        .unwrap_or_else(|error| panic!("should write {}: {error}", path.display()));
}

fn create_materialize_fixture(temp_dir: &TempDir) -> (PathBuf, PathBuf) {
    let flowhub_root = temp_dir.path().join("flowhub");
    let scenario_manifest = temp_dir.path().join("scenario/qianji.toml");

    write_file(
        &flowhub_root.join("blueprint/qianji.toml"),
        r#"
version = 1

[module]
name = "blueprint"
tags = ["planning", "blueprint"]

[exports]
entry = "task.blueprint-start"
ready = "task.blueprint-ready"

[contract]
required = ["template", "template/qianji.toml", "template/*.md"]

[[validation]]
scope = "module"
path = "template"
kind = "dir"
required = true

[[validation]]
scope = "module"
path = "template/qianji.toml"
kind = "file"
required = true

[[validation]]
scope = "module"
path = "template/*.md"
kind = "glob"
min_matches = 1
"#,
    );
    write_file(
        &flowhub_root.join("blueprint/template/qianji.toml"),
        "name = \"blueprint\"\n",
    );
    write_file(
        &flowhub_root.join("blueprint/template/01-blueprint.md"),
        "# Blueprint\n",
    );

    write_file(
        &flowhub_root.join("plan/qianji.toml"),
        r#"
version = 1

[module]
name = "plan"
tags = ["planning", "plan"]

[exports]
entry = "task.plan-start"
ready = "task.plan-ready"

[contract]
required = ["template", "template/qianji.toml", "template/*.md"]

[[validation]]
scope = "module"
path = "template"
kind = "dir"
required = true

[[validation]]
scope = "module"
path = "template/qianji.toml"
kind = "file"
required = true

[[validation]]
scope = "module"
path = "template/*.md"
kind = "glob"
min_matches = 1
"#,
    );
    write_file(
        &flowhub_root.join("plan/template/qianji.toml"),
        "name = \"plan\"\n",
    );
    write_file(&flowhub_root.join("plan/template/01-plan.md"), "# Plan\n");

    write_file(
        &scenario_manifest,
        r#"
version = 1

[planning]
name = "blueprint-plan-demo"

[template]
use = [
  "blueprint as blueprint",
  "plan as plan",
]

[[template.link]]
from = "blueprint::task.blueprint-ready"
to = "plan::task.plan-start"
"#,
    );

    (flowhub_root, scenario_manifest)
}

fn create_unlinked_materialize_fixture(temp_dir: &TempDir) -> (PathBuf, PathBuf) {
    let (flowhub_root, scenario_manifest) = create_materialize_fixture(temp_dir);
    write_file(
        &scenario_manifest,
        r#"
version = 1

[planning]
name = "blueprint-plan-demo"

[template]
use = [
  "blueprint as blueprint",
  "plan as plan",
]
"#,
    );

    (flowhub_root, scenario_manifest)
}

#[test]
fn materialize_flowhub_scenario_generates_compact_work_surface() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let output_dir = temp_dir.path().join("materialized");
    let (fixture_flowhub_root, scenario_manifest) = create_materialize_fixture(&temp_dir);

    let materialized =
        materialize_flowhub_scenario_workdir(fixture_flowhub_root, scenario_manifest, &output_dir)
            .unwrap_or_else(|error| panic!("scenario should materialize: {error}"));

    assert_eq!(materialized.plan_name, "blueprint-plan-demo");
    assert_eq!(
        materialized.visible_aliases,
        vec!["blueprint".to_string(), "plan".to_string()]
    );
    assert!(output_dir.join("qianji.toml").is_file());
    assert!(output_dir.join("flowchart.mmd").is_file());
    assert!(output_dir.join("blueprint/qianji.toml").is_file());
    assert!(output_dir.join("plan/qianji.toml").is_file());
    assert!(output_dir.join("blueprint/01-blueprint.md").is_file());
    assert!(output_dir.join("plan/01-plan.md").is_file());
    assert!(!output_dir.join("rust").exists());

    let flowchart = fs::read_to_string(output_dir.join("flowchart.mmd"))
        .unwrap_or_else(|error| panic!("should read materialized flowchart: {error}"));
    assert!(flowchart.contains("blueprint --> plan"));

    let show = show_workdir(&output_dir)
        .unwrap_or_else(|error| panic!("materialized workdir should show: {error}"));
    assert_eq!(show.plan_name, "blueprint-plan-demo");

    let report = check_workdir(&output_dir)
        .unwrap_or_else(|error| panic!("materialized workdir should check: {error}"));
    assert!(report.is_valid());
}

#[test]
fn materialize_flowhub_scenario_rejects_non_empty_output_dir() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let output_dir = temp_dir.path().join("materialized");
    let (fixture_flowhub_root, scenario_manifest) = create_materialize_fixture(&temp_dir);
    fs::create_dir_all(&output_dir)
        .unwrap_or_else(|error| panic!("should create output dir: {error}"));
    fs::write(output_dir.join("stale.txt"), "stale")
        .unwrap_or_else(|error| panic!("should write stale file: {error}"));

    let error =
        materialize_flowhub_scenario_workdir(fixture_flowhub_root, scenario_manifest, &output_dir)
            .err()
            .unwrap_or_else(|| panic!("non-empty output dir should fail"));

    assert!(error.to_string().contains("must be empty"));
}

#[test]
fn materialize_flowhub_scenario_reports_follow_up_query_for_invalid_generated_surface() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let output_dir = temp_dir.path().join("materialized");
    let (fixture_flowhub_root, scenario_manifest) = create_unlinked_materialize_fixture(&temp_dir);

    let error =
        materialize_flowhub_scenario_workdir(fixture_flowhub_root, scenario_manifest, &output_dir)
            .err()
            .unwrap_or_else(|| panic!("unlinked materialized work surface should fail"));

    let rendered = error.to_string();
    assert!(rendered.contains("Generated work surface"));
    assert!(rendered.contains("# Validation Failed"));
    assert!(rendered.contains("Missing flowchart backbone"));
    assert!(rendered.contains("## Follow-up Query"));
    assert_eq!(rendered.matches("## Follow-up Query").count(), 1);
    assert!(rendered.contains("Surfaces: blueprint, plan"));
    assert!(rendered.contains(
        "select path, surface, heading_path, skeleton \
from markdown \
where surface in ('blueprint', 'plan') \
order by surface, path, heading_path"
    ));
}

xiuxian_testing::crate_test_policy_harness!();
