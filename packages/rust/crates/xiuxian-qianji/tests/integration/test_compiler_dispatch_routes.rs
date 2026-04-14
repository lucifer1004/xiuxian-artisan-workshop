//! Integration tests for compiler dispatch route coverage.

use std::path::Path;
use std::sync::Arc;
use xiuxian_qianhuan::{orchestrator::ThousandFacesOrchestrator, persona::PersonaRegistry};
use xiuxian_qianji::QianjiCompiler;
use xiuxian_wendao::LinkGraphIndex;

const KNOWLEDGE_MANIFEST: &str = r#"
name = "KnowledgeDispatch"

[[nodes]]
id = "Knowledge"
task_type = "knowledge"
weight = 1.0
params = {}
"#;

const COMMAND_MANIFEST: &str = r#"
name = "CommandDispatch"

[[nodes]]
id = "Command"
task_type = "command"
weight = 1.0
params = { cmd = "echo hi", output_key = "stdout" }
"#;

const HTTP_CALL_MANIFEST: &str = r#"
name = "HttpCallDispatch"

[[nodes]]
id = "OpenNavigation"
kind = "http_call"
contract = "wendao.docs.navigation"
method = "GET"
path = "/api/docs/navigation"
query = { repo = "$repo", page_id = "$page_id", related_limit = 5, family_limit = 3 }
"#;

const CLI_CALL_MANIFEST: &str = r#"
name = "CliCallDispatch"

[[nodes]]
id = "OpenNavigationCli"
kind = "cli_call"
contract = "wendao.docs.navigation"
argv = ["wendao", "docs", "navigation", "--repo", "$repo", "--page-id", "$page_id", "--related-limit", "5", "--family-limit", "3"]
"#;

const HTTP_CALL_INVALID_PATH_MANIFEST: &str = r#"
name = "HttpCallInvalidPathDispatch"

[[nodes]]
id = "OpenNavigation"
kind = "http_call"
contract = "wendao.docs.navigation"
method = "GET"
path = "/api/docs/not-navigation"
query = { repo = "$repo", page_id = "$page_id", related_limit = 5, family_limit = 3 }
"#;

const CLI_CALL_UNKNOWN_FLAG_MANIFEST: &str = r#"
name = "CliCallInvalidFlagDispatch"

[[nodes]]
id = "OpenNavigationCli"
kind = "cli_call"
contract = "wendao.docs.navigation"
argv = ["wendao", "docs", "navigation", "--repo", "$repo", "--page-id", "$page_id", "--nope", "3"]
"#;

const WRITE_FILE_MANIFEST: &str = r#"
name = "WriteFileDispatch"

[[nodes]]
id = "WriteFile"
task_type = "write_file"
weight = 1.0
params = { path = "notes/out.txt", content = "hello", output_key = "write_file_result" }
"#;

const SUSPEND_MANIFEST: &str = r#"
name = "SuspendDispatch"

[[nodes]]
id = "Suspend"
task_type = "suspend"
weight = 1.0
params = { reason = "manual-check", prompt = "continue?", resume_key = "resume" }
"#;

const ROUTER_MANIFEST: &str = r#"
name = "RouterDispatch"

[[nodes]]
id = "Router"
task_type = "router"
weight = 1.0
params = { branches = [["A", 0.6], ["B", 0.4]] }
"#;

const ROUTER_INVALID_WEIGHT_MANIFEST: &str = r#"
name = "RouterInvalidWeightDispatch"

[[nodes]]
id = "Router"
task_type = "router"
weight = 1.0
params = { branches = [["A", "not-a-number"]] }
"#;

const CALIBRATION_MANIFEST: &str = r#"
name = "CalibrationDispatch"

[[nodes]]
id = "Calibration"
task_type = "calibration"
weight = 1.0
params = { target_node_id = "TargetNode" }
"#;

const MOCK_MANIFEST: &str = r#"
name = "MockDispatch"

[[nodes]]
id = "MockNode"
task_type = "mock"
weight = 1.0
params = {}
"#;

const SECURITY_SCAN_MANIFEST: &str = r#"
name = "SecurityScanDispatch"

[[nodes]]
id = "SecurityScan"
task_type = "security_scan"
weight = 1.0
params = { files_key = "changed_files", output_key = "issues", abort_on_violation = true }
"#;

const ANNOTATION_EXPLICIT_AFFINITY_MANIFEST: &str = r#"
name = "AnnotationExplicitAffinityDispatch"

[[nodes]]
id = "Annotator"
task_type = "annotation"
weight = 1.0
params = { agent_id = "agent-alpha", role_class = "planner" }
"#;

const ANNOTATION_DERIVED_AFFINITY_MANIFEST: &str = r#"
name = "AnnotationDerivedAffinityDispatch"

[[nodes]]
id = "Annotator"
task_type = "annotation"
weight = 1.0
params = {}
[nodes.qianhuan]
persona_id = "semantic://personas/Steward.md"
"#;

const FORMAL_AUDIT_NATIVE_MANIFEST: &str = r#"
name = "FormalAuditNativeDispatch"

[[nodes]]
id = "Teacher"
task_type = "formal_audit"
weight = 1.0
params = { retry_targets = ["Steward"] }
"#;

const FORMAL_AUDIT_NATIVE_WITH_MAX_RETRIES_MANIFEST: &str = r#"
name = "FormalAuditNativeWithMaxRetriesDispatch"

[[nodes]]
id = "Teacher"
task_type = "formal_audit"
weight = 1.0
params = { retry_targets = ["Steward"], max_retries = 2 }
"#;

#[cfg(not(feature = "llm"))]
const LLM_TASK_MANIFEST: &str = r#"
name = "LlmDispatch"

[[nodes]]
id = "Analyzer"
task_type = "llm"
weight = 1.0
params = { prompt = "Analyze", output_key = "analysis" }
"#;

const WENDAO_INGESTER_MANIFEST: &str = r#"
name = "WendaoIngesterDispatch"

[[nodes]]
id = "WendaoIngester"
task_type = "wendao_ingester"
weight = 1.0
params = {}
"#;

const WENDAO_REFRESH_MANIFEST: &str = r#"
name = "WendaoRefreshDispatch"

[[nodes]]
id = "WendaoRefresh"
task_type = "wendao_refresh"
weight = 1.0
params = {}
"#;

const WENDAO_SQL_DISCOVER_MANIFEST: &str = r#"
name = "WendaoSqlDiscoverDispatch"

[[nodes]]
id = "WendaoSqlDiscover"
task_type = "wendao_sql_discover"
weight = 1.0
params = { endpoint = "http://127.0.0.1:39001/query" }
"#;

const WENDAO_SQL_VALIDATE_MANIFEST: &str = r#"
name = "WendaoSqlValidateDispatch"

[[nodes]]
id = "WendaoSqlValidate"
task_type = "wendao_sql_validate"
weight = 1.0
params = {}
"#;

const WENDAO_SQL_EXECUTE_MANIFEST: &str = r#"
name = "WendaoSqlExecuteDispatch"

[[nodes]]
id = "WendaoSqlExecute"
task_type = "wendao_sql_execute"
weight = 1.0
params = { endpoint = "http://127.0.0.1:39001/query" }
"#;

#[cfg(not(feature = "llm"))]
const FORMAL_AUDIT_LLM_MANIFEST: &str = r#"
name = "FormalAuditDispatch"

[[nodes]]
id = "Teacher"
task_type = "formal_audit"
weight = 1.0
params = { retry_targets = ["Steward"] }
[nodes.qianhuan]
persona_id = "strict_teacher"
template_target = "critique_agenda.j2"
[nodes.llm]
provider = "openai"
model = "gpt-4o-mini"
"#;

const UNKNOWN_TASK_MANIFEST: &str = r#"
name = "UnknownDispatch"

[[nodes]]
id = "Unknown"
task_type = "not_real_task"
weight = 1.0
params = {}
"#;

fn build_compiler(index_root: &Path) -> Result<QianjiCompiler, Box<dyn std::error::Error>> {
    let index = Arc::new(LinkGraphIndex::build(index_root)?);
    let orchestrator = Arc::new(ThousandFacesOrchestrator::new("Rules".to_string(), None));
    let registry = Arc::new(PersonaRegistry::with_builtins());
    Ok(QianjiCompiler::new(index, orchestrator, registry, None))
}

xiuxian_testing::crate_test_policy_harness!();

#[test]
fn compiler_dispatches_knowledge_task_via_stateless_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(KNOWLEDGE_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_command_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(COMMAND_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_http_call_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(HTTP_CALL_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_cli_call_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(CLI_CALL_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_rejects_http_call_when_contract_path_drifts() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = match compiler.compile(HTTP_CALL_INVALID_PATH_MANIFEST) {
        Ok(_) => panic!("invalid HTTP path should fail contract validation"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("/api/docs/navigation"));
    Ok(())
}

#[test]
fn compiler_rejects_cli_call_when_flag_is_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = match compiler.compile(CLI_CALL_UNKNOWN_FLAG_MANIFEST) {
        Ok(_) => panic!("unknown CLI flag should fail contract validation"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("--nope"));
    Ok(())
}

#[test]
fn compiler_dispatches_write_file_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(WRITE_FILE_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_suspend_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(SUSPEND_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_router_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(ROUTER_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_rejects_router_with_invalid_branch_weight() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = compiler
        .compile(ROUTER_INVALID_WEIGHT_MANIFEST)
        .err()
        .unwrap_or_else(|| panic!("router manifest should fail on invalid branch weight"));
    let message = error.to_string();
    assert!(message.contains("Router branch weight"));
    Ok(())
}

#[test]
fn compiler_dispatches_wendao_ingester_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(WENDAO_INGESTER_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_wendao_refresh_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(WENDAO_REFRESH_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_wendao_sql_discover_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(WENDAO_SQL_DISCOVER_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_wendao_sql_validate_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(WENDAO_SQL_VALIDATE_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_wendao_sql_execute_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(WENDAO_SQL_EXECUTE_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_calibration_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(CALIBRATION_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_mock_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(MOCK_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_security_scan_task_via_leaf_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(SECURITY_SCAN_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_dispatches_annotation_and_keeps_explicit_execution_affinity()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(ANNOTATION_EXPLICIT_AFFINITY_MANIFEST)?;

    assert_eq!(engine.graph.node_count(), 1);
    let node_index = engine
        .graph
        .node_indices()
        .next()
        .unwrap_or_else(|| panic!("compiled graph should contain one node"));
    let node = &engine.graph[node_index];
    assert_eq!(
        node.execution_affinity.agent_id.as_deref(),
        Some("agent-alpha")
    );
    assert_eq!(
        node.execution_affinity.role_class.as_deref(),
        Some("planner")
    );
    Ok(())
}

#[test]
fn compiler_dispatches_annotation_and_derives_role_class_from_persona_id()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(ANNOTATION_DERIVED_AFFINITY_MANIFEST)?;

    assert_eq!(engine.graph.node_count(), 1);
    let node_index = engine
        .graph
        .node_indices()
        .next()
        .unwrap_or_else(|| panic!("compiled graph should contain one node"));
    let node = &engine.graph[node_index];
    assert_eq!(node.execution_affinity.agent_id, None);
    assert_eq!(
        node.execution_affinity.role_class.as_deref(),
        Some("steward")
    );
    Ok(())
}

#[test]
fn compiler_dispatches_formal_audit_native_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let engine = compiler.compile(FORMAL_AUDIT_NATIVE_MANIFEST)?;
    assert_eq!(engine.graph.node_count(), 1);
    Ok(())
}

#[test]
fn compiler_rejects_native_formal_audit_with_max_retries_without_llm_controller()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = compiler
        .compile(FORMAL_AUDIT_NATIVE_WITH_MAX_RETRIES_MANIFEST)
        .err()
        .unwrap_or_else(|| panic!("native formal_audit with max_retries should fail"));
    let message = error.to_string();
    assert!(message.contains("formal_audit.max_retries"));
    assert!(message.contains("native formal_audit"));
    Ok(())
}

#[cfg(not(feature = "llm"))]
#[test]
fn compiler_rejects_llm_augmented_formal_audit_without_llm_feature()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = compiler
        .compile(FORMAL_AUDIT_LLM_MANIFEST)
        .err()
        .unwrap_or_else(|| panic!("manifest should fail without llm feature"));
    let message = error.to_string();
    assert!(message.contains("Task type `formal_audit`"));
    assert!(message.contains("feature `llm`"));
    Ok(())
}

#[cfg(not(feature = "llm"))]
#[test]
fn compiler_rejects_llm_task_without_llm_feature() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = compiler
        .compile(LLM_TASK_MANIFEST)
        .err()
        .unwrap_or_else(|| panic!("llm task manifest should fail without llm feature"));
    let message = error.to_string();
    assert!(message.contains("Task type 'llm'"));
    assert!(message.contains("feature 'llm'"));
    Ok(())
}

#[test]
fn compiler_rejects_unknown_task_type_with_topology_error() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let compiler = build_compiler(temp.path())?;
    let error = compiler
        .compile(UNKNOWN_TASK_MANIFEST)
        .err()
        .unwrap_or_else(|| panic!("unknown task type manifest should fail"));
    let message = error.to_string();
    assert!(message.contains("Unknown task type: not_real_task"));
    Ok(())
}
