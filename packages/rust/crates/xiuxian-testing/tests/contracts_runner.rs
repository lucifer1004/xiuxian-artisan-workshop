//! Integration tests for the executable contract-suite runner.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use futures::executor::block_on;
use serde_json::json;
use xiuxian_testing::{
    AdvisoryAuditExecutor, AdvisoryAuditPolicy, AdvisoryAuditRequest, ArtifactKind,
    CollectedArtifact, CollectedArtifacts, CollectionContext, ContractExecutionMode,
    ContractFinding, ContractRunConfig, ContractSuite, ContractSuiteRunner, FindingSeverity,
    NoopAdvisoryAuditExecutor, RoleAuditFinding, RulePack, RulePackDescriptor,
};

#[derive(Debug, Clone, Copy)]
struct FakeRulePack;

impl RulePack for FakeRulePack {
    fn descriptor(&self) -> RulePackDescriptor {
        RulePackDescriptor {
            id: "rest_docs",
            version: "v1",
            domains: &["rest", "documentation"],
            default_mode: xiuxian_testing::FindingMode::Deterministic,
        }
    }

    fn collect(&self, _ctx: &CollectionContext) -> Result<CollectedArtifacts> {
        let mut artifacts = CollectedArtifacts::default();
        artifacts.push(CollectedArtifact {
            id: "openapi".to_string(),
            kind: ArtifactKind::OpenApiDocument,
            path: None,
            content: json!({
                "openapi": "3.1.0",
                "paths": {}
            }),
            labels: std::collections::BTreeMap::new(),
        });
        Ok(artifacts)
    }

    fn evaluate(&self, _artifacts: &CollectedArtifacts) -> Result<Vec<ContractFinding>> {
        Ok(vec![ContractFinding::new(
            "REST-R001",
            "rest_docs",
            FindingSeverity::Error,
            xiuxian_testing::FindingMode::Deterministic,
            "Missing endpoint purpose",
            "The endpoint is missing a purpose description.",
        )])
    }
}

#[derive(Debug, Default, Clone)]
struct RecordingAdvisoryExecutor {
    requests: Arc<Mutex<Vec<AdvisoryAuditRequest>>>,
}

impl RecordingAdvisoryExecutor {
    fn recorded_requests(&self) -> Vec<AdvisoryAuditRequest> {
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        requests.clone()
    }
}

#[async_trait]
impl AdvisoryAuditExecutor for RecordingAdvisoryExecutor {
    async fn run(&self, request: AdvisoryAuditRequest) -> Result<Vec<RoleAuditFinding>> {
        {
            let mut requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            requests.push(request.clone());
        }

        let mut finding = RoleAuditFinding::new(
            "rest_contract_auditor",
            FindingSeverity::Warning,
            "The endpoint contract should include a business-facing purpose.",
        )
        .with_trace_id("trace-42");
        finding.rule_id = Some("AUDIT-R001".to_string());
        finding.why_it_matters =
            "Without a purpose statement, downstream readers cannot infer endpoint intent."
                .to_string();
        finding.remediation =
            "Add a short summary plus one example request for the endpoint.".to_string();
        finding.push_message_evidence("deterministic rule REST-R001 triggered advisory review");
        finding
            .examples
            .good
            .push("summary: Creates a knowledge node.".to_string());
        finding.examples.bad.push("summary: <missing>".to_string());

        Ok(vec![finding])
    }
}

fn test_context() -> CollectionContext {
    let mut context = CollectionContext {
        suite_id: "contracts".to_string(),
        crate_name: Some("xiuxian-wendao".to_string()),
        workspace_root: Some(workspace_root()),
        labels: std::collections::BTreeMap::new(),
    };
    context
        .labels
        .insert("lane".to_string(), "research".to_string());
    context
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap_or_else(|| panic!("testing manifest dir should resolve to workspace root"))
        .to_path_buf()
}

#[test]
fn suite_runner_keeps_deterministic_findings_when_advisory_is_disabled() {
    let mut suite = ContractSuite::new("contracts", "v1");
    suite.register_rule_pack(Box::new(FakeRulePack));

    let runner = ContractSuiteRunner::new(&NoopAdvisoryAuditExecutor);
    let config = ContractRunConfig {
        execution_mode: ContractExecutionMode::Advisory,
        generated_at: "2026-03-17T00:00:00Z".to_string(),
        ..ContractRunConfig::default()
    };

    let report = match block_on(runner.run(&suite, &test_context(), &config)) {
        Ok(report) => report,
        Err(error) => panic!("deterministic-only suite run should succeed: {error}"),
    };

    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.stats.total, 1);
    assert_eq!(report.stats.deterministic, 1);
    assert_eq!(report.stats.advisory, 0);
}

#[test]
fn suite_runner_merges_advisory_findings_and_preserves_request_context() {
    let mut suite = ContractSuite::new("contracts", "v1");
    suite.register_rule_pack(Box::new(FakeRulePack));

    let executor = RecordingAdvisoryExecutor::default();
    let runner = ContractSuiteRunner::new(&executor);
    let mut config = ContractRunConfig {
        execution_mode: ContractExecutionMode::Advisory,
        generated_at: "2026-03-17T01:00:00Z".to_string(),
        ..ContractRunConfig::default()
    };
    config.set_advisory_policy_for_pack(
        "rest_docs",
        AdvisoryAuditPolicy {
            enabled: true,
            requested_roles: vec![
                "rest_contract_auditor".to_string(),
                "runtime_trace_reviewer".to_string(),
            ],
            min_severity: FindingSeverity::Warning,
        },
    );

    let report = match block_on(runner.run(&suite, &test_context(), &config)) {
        Ok(report) => report,
        Err(error) => panic!("suite run with advisory merge should succeed: {error}"),
    };

    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.stats.total, 2);
    assert_eq!(report.stats.deterministic, 1);
    assert_eq!(report.stats.advisory, 1);

    let Some(advisory_finding) = report
        .findings
        .iter()
        .find(|finding| finding.mode == xiuxian_testing::FindingMode::Advisory)
    else {
        panic!("report should contain one advisory finding");
    };
    assert_eq!(advisory_finding.pack_id, "rest_docs");
    assert_eq!(advisory_finding.rule_id, "AUDIT-R001");
    assert_eq!(
        advisory_finding.advisory_role_ids,
        vec!["rest_contract_auditor".to_string()]
    );
    assert_eq!(advisory_finding.trace_ids, vec!["trace-42".to_string()]);
    assert_eq!(
        advisory_finding
            .labels
            .get("source_lane")
            .map(String::as_str),
        Some("advisory")
    );

    let requests = executor.recorded_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].suite_id, "contracts");
    assert_eq!(requests[0].pack_id, "rest_docs");
    assert_eq!(requests[0].pack_version, "v1");
    assert_eq!(
        requests[0].pack_domains,
        vec!["rest".to_string(), "documentation".to_string()]
    );
    assert_eq!(
        requests[0].collection_context.crate_name.as_deref(),
        Some("xiuxian-wendao")
    );
    assert_eq!(
        requests[0]
            .collection_context
            .labels
            .get("lane")
            .map(String::as_str),
        Some("research")
    );
    assert_eq!(
        requests[0].requested_roles,
        vec![
            "rest_contract_auditor".to_string(),
            "runtime_trace_reviewer".to_string(),
        ]
    );
}
