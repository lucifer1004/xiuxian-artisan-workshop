use super::{
    ContractFeedbackCliCommand, DEFAULT_CONTRACT_FEEDBACK_TABLE_NAME, DirCliCommand,
    REST_DOCS_PACK_ID, RestDocsCliCommand, build_contract_feedback_config,
    build_rest_docs_collection_context, parse_contract_feedback_command, parse_dir_command,
    resolve_workspace_root, run_deterministic_rest_docs_contract_feedback, run_dir_command,
    run_scaffold_rest_docs_contract_feedback, sanitize_prj_cache_home,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use xiuxian_config_core::{resolve_cache_home_from_value, resolve_project_root};

fn to_args(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

fn must_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

fn must_some<T>(value: Option<T>, context: &str) -> T {
    value.unwrap_or_else(|| panic!("{context}"))
}

fn write_openapi_fixture(temp_dir: &TempDir) -> PathBuf {
    let path = temp_dir.path().join("openapi.yaml");
    let content = r#"
openapi: 3.1.0
paths:
  /api/search:
    get:
      responses:
        "200":
          description: ok
"#;
    must_ok(
        fs::write(&path, content),
        "should write temporary OpenAPI fixture",
    );
    path
}

fn rest_docs_command(openapi_path: &Path, workspace_root: &Path) -> RestDocsCliCommand {
    RestDocsCliCommand {
        openapi_path: openapi_path.to_path_buf(),
        workspace_root: Some(workspace_root.to_path_buf()),
        storage_path: None,
        table_name: DEFAULT_CONTRACT_FEEDBACK_TABLE_NAME.to_string(),
        no_persist: true,
        live_advisory: false,
        roles: Vec::new(),
        model: None,
        temperature: None,
        cognitive_early_halt_threshold: None,
    }
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        must_ok(
            fs::create_dir_all(parent),
            "should create workdir fixture parent directories",
        );
    }
    must_ok(
        fs::write(path, content),
        "should write workdir fixture file",
    );
}

fn create_workdir_fixture(temp_dir: &TempDir) -> PathBuf {
    let workdir = temp_dir.path().join("demo-plan");
    must_ok(
        fs::create_dir_all(&workdir),
        "should create temporary workdir fixture root",
    );
    write_file(
        &workdir.join("qianji.toml"),
        r#"
version = 1

[plan]
name = "demo-plan"
surface = ["flowchart.mmd", "blueprint", "plan"]

[check]
require = ["flowchart.mmd", "blueprint", "plan", "blueprint/**/*.md", "plan/**/*.md"]
flowchart = ["blueprint", "plan"]
"#,
    );
    write_file(
        &workdir.join("flowchart.mmd"),
        "flowchart LR\n  blueprint --> plan\n",
    );
    write_file(
        &workdir.join("blueprint/architecture.md"),
        "# Blueprint\n\n## Boundary\n\n- [ ] define boundary\n",
    );
    write_file(
        &workdir.join("plan/tasks.md"),
        "# Plan\n\n## Rust\n\n- [ ] implement\n",
    );
    workdir
}

fn repo_root() -> PathBuf {
    resolve_project_root()
        .unwrap_or_else(|| panic!("workspace root should resolve from PRJ_ROOT or git ancestry"))
}

fn flowhub_root() -> PathBuf {
    repo_root().join("qianji-flowhub")
}

fn scenario_fixture_dir(name: &str) -> PathBuf {
    repo_root().join(format!(
        "packages/rust/crates/xiuxian-qianji/tests/fixtures/flowhub/{name}"
    ))
}

fn create_invalid_scenario_fixture(temp_dir: &TempDir) -> PathBuf {
    let scenario_dir = temp_dir.path().join("scenario");
    must_ok(
        fs::create_dir_all(&scenario_dir),
        "should create scenario fixture root",
    );
    write_file(
        &scenario_dir.join("qianji.toml"),
        r#"
version = 1

[planning]
name = "broken-scenario"

[template]
use = ["missing-module as missing"]
"#,
    );
    scenario_dir
}

fn default_contract_feedback_storage_path_with(
    workspace_root: &Path,
    raw_cache_home: Option<&str>,
) -> PathBuf {
    resolve_prj_cache_home_with(workspace_root, raw_cache_home)
        .join("wendao")
        .join("contract_feedback")
}

fn resolve_prj_cache_home_with(workspace_root: &Path, raw_cache_home: Option<&str>) -> PathBuf {
    let resolved = resolve_cache_home_from_value(Some(workspace_root), raw_cache_home)
        .unwrap_or_else(|| workspace_root.join(".cache"));
    sanitize_prj_cache_home(workspace_root, resolved)
}

#[test]
fn parse_rest_docs_contract_feedback_command_uses_defaults() {
    let command = must_some(
        must_ok(
            parse_contract_feedback_command(&to_args(&[
                "qianji",
                "contract-feedback",
                "rest-docs",
                "specs/openapi.yaml",
            ])),
            "contract-feedback parse should succeed",
        ),
        "command should be detected",
    );

    let ContractFeedbackCliCommand::RestDocs(command) = command;
    assert_eq!(command.openapi_path, PathBuf::from("specs/openapi.yaml"));
    assert_eq!(command.table_name, DEFAULT_CONTRACT_FEEDBACK_TABLE_NAME);
    assert!(!command.no_persist);
    assert!(!command.live_advisory);
    assert!(command.roles.is_empty());
}

#[test]
fn parse_rest_docs_contract_feedback_command_supports_advisory_flags() {
    let command = must_some(
        must_ok(
            parse_contract_feedback_command(&to_args(&[
                "qianji",
                "contract-feedback",
                "rest-docs",
                "specs/openapi.yaml",
                "--workspace-root",
                "/tmp/workspace",
                "--storage-path",
                ".cache/wendao",
                "--table-name",
                "contract_audit",
                "--role",
                "strict_teacher",
                "--role",
                "rest_contract_auditor",
                "--live-advisory",
                "--temperature",
                "0.2",
                "--cognitive-threshold",
                "0.35",
            ])),
            "contract-feedback parse should succeed",
        ),
        "command should be detected",
    );

    let ContractFeedbackCliCommand::RestDocs(command) = command;
    assert_eq!(
        command.workspace_root,
        Some(PathBuf::from("/tmp/workspace"))
    );
    assert_eq!(command.storage_path, Some(PathBuf::from(".cache/wendao")));
    assert_eq!(command.table_name, "contract_audit");
    assert_eq!(
        command.roles,
        vec![
            "strict_teacher".to_string(),
            "rest_contract_auditor".to_string()
        ]
    );
    assert!(command.live_advisory);
    assert_eq!(command.temperature, Some(0.2));
    assert_eq!(command.cognitive_early_halt_threshold, Some(0.35));
}

#[test]
fn parse_show_workdir_command_requires_dir_flag() {
    let command = must_some(
        must_ok(
            parse_dir_command(&to_args(&["qianji", "show", "--dir", "/tmp/workdir"])),
            "show parse should succeed",
        ),
        "show command should be detected",
    );

    assert_eq!(
        command,
        DirCliCommand::Show {
            dir: PathBuf::from("/tmp/workdir")
        }
    );
}

#[test]
fn parse_check_workdir_command_requires_dir_flag() {
    let command = must_some(
        must_ok(
            parse_dir_command(&to_args(&["qianji", "check", "--dir", "demo"])),
            "check parse should succeed",
        ),
        "check command should be detected",
    );

    assert_eq!(
        command,
        DirCliCommand::Check {
            dir: PathBuf::from("demo")
        }
    );
}

fn assert_common_show_shape(rendered: &str) {
    assert!(rendered.starts_with("# "));
    assert!(rendered.contains("Location:"));
    assert!(rendered.contains("\n## "));
}

#[test]
fn run_show_workdir_command_renders_surface_summary() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let workdir = create_workdir_fixture(&temp_dir);

    let output = must_ok(
        run_dir_command(DirCliCommand::Show {
            dir: workdir.clone(),
        }),
        "show command should render",
    );

    assert_eq!(output.exit_code, 0);
    assert_common_show_shape(&output.rendered);
    assert!(output.rendered.contains("# Work Surface"));
    assert!(output.rendered.contains("## blueprint"));
    assert!(output.rendered.contains("- architecture.md"));
    assert!(output.rendered.contains("## plan"));
    assert!(output.rendered.contains("- tasks.md"));
}

#[test]
fn run_check_workdir_command_blocks_invalid_surface() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let workdir = create_workdir_fixture(&temp_dir);
    must_ok(
        fs::remove_file(workdir.join("plan/tasks.md")),
        "should remove plan markdown for failing check",
    );

    let output = must_ok(
        run_dir_command(DirCliCommand::Check { dir: workdir }),
        "check command should render diagnostics",
    );

    assert_eq!(output.exit_code, 2);
    assert!(output.rendered.contains("# Validation Failed"));
    assert!(output.rendered.contains("Missing required glob matches"));
    assert!(output.rendered.contains("## Follow-up Query"));
    assert!(output.rendered.contains("Surfaces: plan"));
}

#[test]
fn run_check_workdir_command_renders_follow_up_query() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let workdir = create_workdir_fixture(&temp_dir);
    must_ok(
        fs::remove_file(workdir.join("plan/tasks.md")),
        "should remove plan markdown for follow-up rendering",
    );

    let output = must_ok(
        run_dir_command(DirCliCommand::Check { dir: workdir }),
        "check command should render follow-up query",
    );

    assert_eq!(output.exit_code, 2);
    assert!(output.rendered.contains("## Follow-up Query"));
    assert!(output.rendered.contains(
        "select path, surface, heading_path, skeleton \
from markdown \
where surface = 'plan' \
order by surface, path, heading_path"
    ));
}

#[test]
fn run_show_dir_command_renders_flowhub_summary() {
    let output = must_ok(
        run_dir_command(DirCliCommand::Show {
            dir: flowhub_root(),
        }),
        "show command should render Flowhub summary",
    );

    assert_eq!(output.exit_code, 0);
    assert_common_show_shape(&output.rendered);
    assert!(output.rendered.contains("# Flowhub"));
    assert!(output.rendered.contains("## rust"));
    assert!(output.rendered.contains("## blueprint"));
}

#[test]
fn run_show_dir_command_renders_scenario_preview() {
    let output = must_ok(
        run_dir_command(DirCliCommand::Show {
            dir: scenario_fixture_dir("coding_rust_blueprint_plan"),
        }),
        "show command should render scenario preview",
    );

    assert_eq!(output.exit_code, 0);
    assert_common_show_shape(&output.rendered);
    assert!(output.rendered.contains("# Scenario Work Surface Preview"));
    assert!(
        output
            .rendered
            .contains("Scenario: coding-rust-blueprint-plan-demo")
    );
    assert!(output.rendered.contains("## blueprint"));
    assert!(output.rendered.contains("## plan"));
    assert!(output.rendered.contains("blueprint --> plan"));
}

#[test]
fn run_check_dir_command_accepts_flowhub_root() {
    let output = must_ok(
        run_dir_command(DirCliCommand::Check {
            dir: flowhub_root(),
        }),
        "check command should validate Flowhub root",
    );

    assert_eq!(output.exit_code, 0);
    assert!(output.rendered.contains("# Validation Passed"));
    assert!(output.rendered.contains("Checked modules:"));
}

#[test]
fn run_check_dir_command_accepts_scenario_dir() {
    let output = must_ok(
        run_dir_command(DirCliCommand::Check {
            dir: scenario_fixture_dir("coding_rust_blueprint_plan"),
        }),
        "check command should validate scenario dir",
    );

    assert_eq!(output.exit_code, 0);
    assert!(output.rendered.contains("# Validation Passed"));
    assert!(
        output
            .rendered
            .contains("Scenario: coding-rust-blueprint-plan-demo")
    );
    assert!(
        output
            .rendered
            .contains("Visible surfaces: flowchart.mmd, coding, rust, blueprint, plan")
    );
}

#[test]
fn run_check_dir_command_blocks_invalid_scenario_dir() {
    let temp_dir =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should allocate: {error}"));
    let scenario_dir = create_invalid_scenario_fixture(&temp_dir);

    let output = must_ok(
        run_dir_command(DirCliCommand::Check { dir: scenario_dir }),
        "check command should render scenario diagnostics",
    );

    assert_eq!(output.exit_code, 2);
    assert!(output.rendered.contains("# Validation Failed"));
    assert!(output.rendered.contains("Scenario resolve failed"));
    assert!(output.rendered.contains("missing-module"));
}

#[tokio::test]
async fn deterministic_rest_docs_contract_feedback_outputs_expected_summary() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let openapi_path = write_openapi_fixture(&temp_dir);
    let workspace_root = temp_dir.path().to_path_buf();
    let command = rest_docs_command(&openapi_path, &workspace_root);

    let context = build_rest_docs_collection_context(&openapi_path, Some(workspace_root.clone()));
    let config = build_contract_feedback_config(&command);
    let advisory_roles = config
        .advisory_policy_for_pack(REST_DOCS_PACK_ID)
        .requested_roles;
    assert!(advisory_roles.is_empty());

    let output = must_ok(
        run_deterministic_rest_docs_contract_feedback(
            &command,
            &openapi_path,
            workspace_root.as_path(),
            context,
            &config,
            advisory_roles,
        )
        .await,
        "deterministic rest-docs contract feedback should succeed",
    );

    assert_eq!(output.report.suite_id, "qianji-rest-docs-contract-feedback");
    assert_eq!(output.report.stats.total, 2);
    assert_eq!(output.report.stats.deterministic, 2);
    assert_eq!(output.report.stats.advisory, 0);
    assert_eq!(output.knowledge_entry_ids.len(), 2);
    assert!(output.persisted_entry_ids.is_empty());
    assert!(output.storage.is_none());
}

#[tokio::test]
async fn scaffold_rest_docs_contract_feedback_emits_role_advisory_findings() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let openapi_path = write_openapi_fixture(&temp_dir);
    let workspace_root = temp_dir.path().to_path_buf();
    let mut command = rest_docs_command(&openapi_path, &workspace_root);
    command.roles = vec!["strict_teacher".to_string(), "artisan-engineer".to_string()];

    let context = build_rest_docs_collection_context(&openapi_path, Some(workspace_root.clone()));
    let config = build_contract_feedback_config(&command);
    let advisory_roles = config
        .advisory_policy_for_pack(REST_DOCS_PACK_ID)
        .requested_roles;
    assert_eq!(
        advisory_roles,
        vec!["strict_teacher".to_string(), "artisan-engineer".to_string()]
    );

    let output = must_ok(
        run_scaffold_rest_docs_contract_feedback(
            &command,
            &openapi_path,
            workspace_root.as_path(),
            context,
            &config,
            advisory_roles,
        )
        .await,
        "scaffold rest-docs contract feedback should succeed",
    );

    assert_eq!(
        output.advisory_roles,
        vec!["strict_teacher".to_string(), "artisan-engineer".to_string()]
    );
    assert_eq!(output.report.stats.deterministic, 2);
    assert_eq!(output.report.stats.advisory, 2);
    assert_eq!(output.report.stats.total, 4);
    assert_eq!(output.knowledge_entry_ids.len(), 4);
    assert!(output.persisted_entry_ids.is_empty());
    assert!(output.storage.is_none());
}

#[test]
fn default_contract_feedback_storage_path_uses_workspace_cache_root() {
    let workspace_root = Path::new("/repo/workspace");
    let resolved = default_contract_feedback_storage_path_with(workspace_root, None);
    assert_eq!(
        resolved,
        PathBuf::from("/repo/workspace/.cache/wendao/contract_feedback")
    );
}

#[test]
fn resolve_prj_cache_home_resolves_relative_override_against_workspace_root() {
    let resolved =
        resolve_prj_cache_home_with(Path::new("/repo/workspace"), Some(".runtime/cache"));
    assert_eq!(resolved, PathBuf::from("/repo/workspace/.runtime/cache"));
}

#[test]
fn resolve_prj_cache_home_ignores_foreign_absolute_override() {
    let resolved =
        resolve_prj_cache_home_with(Path::new("/repo/workspace"), Some("/tmp/foreign-cache"));
    assert_eq!(resolved, PathBuf::from("/repo/workspace/.cache"));
}

#[test]
fn resolve_workspace_root_prefers_explicit_path() {
    let explicit = Path::new("/tmp/explicit-workspace");
    let resolved = must_ok(
        resolve_workspace_root(Some(explicit)),
        "resolve workspace root",
    );
    assert_eq!(resolved, explicit);
}
