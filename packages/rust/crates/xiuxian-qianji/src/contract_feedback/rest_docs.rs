//! File-backed `rest_docs` contract-feedback helpers for real Qianji callers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use xiuxian_testing::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, CollectionContext, ContractFinding,
    ContractRunConfig, ContractSuite, NoopAdvisoryAuditExecutor, RestDocsRulePack, RulePack,
    RulePackDescriptor,
};

use crate::sovereign::ContractFeedbackKnowledgeSink;

use super::pipeline::{
    QianjiContractFeedbackRun, QianjiPersistedContractFeedbackRun,
    run_and_persist_contract_feedback_flow, run_contract_feedback_flow,
};

const REST_DOCS_SUITE_ID: &str = "qianji-rest-docs-contract-feedback";

/// `rest_docs` rule-pack wrapper that reads one local `OpenAPI` file from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiFileRestDocsRulePack {
    openapi_path: PathBuf,
    artifact_id: String,
}

impl OpenApiFileRestDocsRulePack {
    /// Create a new file-backed `rest_docs` rule pack.
    #[must_use]
    pub fn new(openapi_path: impl Into<PathBuf>) -> Self {
        let openapi_path = openapi_path.into();
        let artifact_id = format!("openapi:{}", openapi_path.display());
        Self {
            openapi_path,
            artifact_id,
        }
    }

    /// Return the backing `OpenAPI` document path.
    #[must_use]
    pub fn openapi_path(&self) -> &Path {
        &self.openapi_path
    }

    fn load_openapi_document(&self) -> Result<serde_json::Value> {
        let raw = fs::read_to_string(&self.openapi_path).with_context(|| {
            format!(
                "failed to read OpenAPI document at {}",
                self.openapi_path.display()
            )
        })?;

        serde_json::from_str(&raw)
            .or_else(|_| serde_yaml::from_str::<serde_json::Value>(&raw))
            .with_context(|| {
                format!(
                    "failed to parse OpenAPI document at {} as JSON or YAML",
                    self.openapi_path.display()
                )
            })
    }
}

impl RulePack for OpenApiFileRestDocsRulePack {
    fn descriptor(&self) -> RulePackDescriptor {
        RestDocsRulePack.descriptor()
    }

    fn collect(&self, _ctx: &CollectionContext) -> Result<CollectedArtifacts> {
        let mut artifacts = CollectedArtifacts::default();
        artifacts.push(CollectedArtifact {
            id: self.artifact_id.clone(),
            kind: ArtifactKind::OpenApiDocument,
            path: Some(self.openapi_path.clone()),
            content: self.load_openapi_document()?,
            labels: BTreeMap::from([("artifact_source".to_string(), "openapi_file".to_string())]),
        });
        Ok(artifacts)
    }

    fn evaluate(&self, artifacts: &CollectedArtifacts) -> Result<Vec<ContractFinding>> {
        RestDocsRulePack.evaluate(artifacts)
    }
}

/// Build a bounded contract suite that evaluates one local `OpenAPI` file with the built-in
/// `rest_docs` pack.
#[must_use]
pub fn build_rest_docs_contract_suite(openapi_path: impl Into<PathBuf>) -> ContractSuite {
    let mut suite = ContractSuite::new(REST_DOCS_SUITE_ID, "v1");
    suite.register_rule_pack(Box::new(OpenApiFileRestDocsRulePack::new(openapi_path)));
    suite
}

/// Build collection context for one file-backed `rest_docs` contract-feedback run.
#[must_use]
pub fn build_rest_docs_collection_context(
    openapi_path: &Path,
    workspace_root: Option<PathBuf>,
) -> CollectionContext {
    CollectionContext {
        suite_id: REST_DOCS_SUITE_ID.to_string(),
        crate_name: None,
        workspace_root,
        labels: BTreeMap::from([
            ("artifact_source".to_string(), "openapi_file".to_string()),
            (
                "openapi_path".to_string(),
                openapi_path.to_string_lossy().into_owned(),
            ),
        ]),
    }
}

/// Run file-backed `rest_docs` contract feedback without persistence.
///
/// # Errors
///
/// Returns an error when the `OpenAPI` file cannot be loaded, when deterministic evaluation
/// fails, or when the advisory executor fails for a triggered advisory lane.
pub async fn run_rest_docs_contract_feedback(
    openapi_path: impl Into<PathBuf>,
    collection_context: CollectionContext,
    config: &ContractRunConfig,
    advisory_executor: &dyn xiuxian_testing::AdvisoryAuditExecutor,
) -> Result<QianjiContractFeedbackRun> {
    let suite = build_rest_docs_contract_suite(openapi_path);
    run_contract_feedback_flow(&suite, &collection_context, config, advisory_executor).await
}

/// Run file-backed `rest_docs` contract feedback and persist the result through the provided sink.
///
/// # Errors
///
/// Returns an error when the `OpenAPI` file cannot be loaded, when the deterministic contract run
/// fails, or when the sink cannot persist the generated knowledge entries.
pub async fn run_and_persist_rest_docs_contract_feedback(
    openapi_path: impl Into<PathBuf>,
    collection_context: CollectionContext,
    config: &ContractRunConfig,
    sink: &dyn ContractFeedbackKnowledgeSink,
) -> Result<QianjiPersistedContractFeedbackRun> {
    let suite = build_rest_docs_contract_suite(openapi_path);
    run_and_persist_contract_feedback_flow(
        &suite,
        &collection_context,
        config,
        &NoopAdvisoryAuditExecutor,
        sink,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        OpenApiFileRestDocsRulePack, build_rest_docs_collection_context,
        build_rest_docs_contract_suite, run_and_persist_rest_docs_contract_feedback,
        run_rest_docs_contract_feedback,
    };
    use crate::sovereign::InMemoryContractFeedbackSink;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use xiuxian_testing::{
        CollectionContext, ContractRunConfig, NoopAdvisoryAuditExecutor, RulePack,
    };

    fn must_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
        result.unwrap_or_else(|error| panic!("{context}: {error}"))
    }

    fn write_openapi_yaml(temp_dir: &TempDir) -> PathBuf {
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

    #[test]
    fn openapi_file_rest_docs_rule_pack_collects_yaml_document() {
        let temp_dir = must_ok(TempDir::new(), "should create temp dir");
        let openapi_path = write_openapi_yaml(&temp_dir);
        let pack = OpenApiFileRestDocsRulePack::new(&openapi_path);

        let artifacts = must_ok(
            pack.collect(&CollectionContext::default()),
            "file-backed rest_docs collect should succeed",
        );

        assert_eq!(artifacts.len(), 1);
        assert_eq!(
            artifacts.artifacts[0].path.as_deref(),
            Some(openapi_path.as_path())
        );
        assert_eq!(
            artifacts.artifacts[0].content["openapi"],
            serde_json::Value::String("3.1.0".to_string())
        );
    }

    #[tokio::test]
    async fn run_and_persist_rest_docs_contract_feedback_uses_sink() {
        let temp_dir = must_ok(TempDir::new(), "should create temp dir");
        let openapi_path = write_openapi_yaml(&temp_dir);
        let ctx =
            build_rest_docs_collection_context(&openapi_path, Some(temp_dir.path().to_path_buf()));
        let sink = InMemoryContractFeedbackSink::new();

        let result = must_ok(
            run_and_persist_rest_docs_contract_feedback(
                &openapi_path,
                ctx,
                &ContractRunConfig::default(),
                &sink,
            )
            .await,
            "run-and-persist rest_docs contract feedback should succeed",
        );

        assert_eq!(
            result.run.report.suite_id,
            "qianji-rest-docs-contract-feedback"
        );
        assert_eq!(result.persisted_entry_ids.len(), 2);
        assert_eq!(sink.len(), 2);
        assert_eq!(
            sink.entries()
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            result.persisted_entry_ids
        );
    }

    #[tokio::test]
    async fn run_rest_docs_contract_feedback_returns_deterministic_report() {
        let temp_dir = must_ok(TempDir::new(), "should create temp dir");
        let openapi_path = write_openapi_yaml(&temp_dir);
        let ctx =
            build_rest_docs_collection_context(&openapi_path, Some(temp_dir.path().to_path_buf()));

        let result = must_ok(
            run_rest_docs_contract_feedback(
                &openapi_path,
                ctx,
                &ContractRunConfig::default(),
                &NoopAdvisoryAuditExecutor,
            )
            .await,
            "rest_docs contract feedback should succeed without persistence",
        );

        assert_eq!(result.report.suite_id, "qianji-rest-docs-contract-feedback");
        assert_eq!(result.report.stats.total, 2);
        assert_eq!(result.knowledge_entries.len(), 2);
    }

    #[test]
    fn build_rest_docs_contract_suite_registers_one_pack() {
        let suite = build_rest_docs_contract_suite("openapi.yaml");
        assert_eq!(suite.id(), "qianji-rest-docs-contract-feedback");
        assert_eq!(suite.rule_pack_count(), 1);
    }
}
