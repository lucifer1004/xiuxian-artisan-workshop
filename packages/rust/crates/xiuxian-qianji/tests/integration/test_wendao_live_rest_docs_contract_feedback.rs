//! Integration coverage for the real Wendao bundled OpenAPI artifact.

use std::path::{Path, PathBuf};

use xiuxian_qianji::contract_feedback::{
    build_rest_docs_collection_context, run_rest_docs_contract_feedback,
};
use xiuxian_testing::{ContractExecutionMode, ContractRunConfig, NoopAdvisoryAuditExecutor};
use xiuxian_wendao::gateway::openapi::bundled_wendao_gateway_openapi_path;

fn must_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(4)
        .unwrap_or_else(|| panic!("qianji manifest dir should resolve to workspace root"))
        .to_path_buf()
}

#[tokio::test]
async fn wendao_bundled_openapi_rest_docs_contract_feedback_stays_clean() {
    let openapi_path = bundled_wendao_gateway_openapi_path();
    assert!(
        openapi_path.is_file(),
        "expected bundled Wendao OpenAPI artifact to exist at {}",
        openapi_path.display()
    );

    let result = must_ok(
        run_rest_docs_contract_feedback(
            &openapi_path,
            build_rest_docs_collection_context(&openapi_path, Some(workspace_root())),
            &ContractRunConfig {
                execution_mode: ContractExecutionMode::Strict,
                generated_at: "2026-03-18T00:00:00Z".to_string(),
                ..ContractRunConfig::default()
            },
            &NoopAdvisoryAuditExecutor,
        )
        .await,
        "rest_docs contract feedback should succeed for the bundled Wendao OpenAPI artifact",
    );

    assert_eq!(result.report.suite_id, "qianji-rest-docs-contract-feedback");
    assert_eq!(result.report.stats.total, 0);
    assert!(result.report.findings.is_empty());
    assert!(result.knowledge_batch.entries.is_empty());
    assert!(result.knowledge_entries.is_empty());
}
