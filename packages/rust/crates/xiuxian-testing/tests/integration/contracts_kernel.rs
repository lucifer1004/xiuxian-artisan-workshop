//! Integration tests for the `xiuxian-testing` contract kernel.

use xiuxian_testing::{
    AdvisoryAuditPolicy, CollectionContext, ContractExecutionMode, ContractFinding, ContractReport,
    ContractStats, ContractSuite, FindingMode, FindingSeverity, NoopRulePack, RoleAuditFinding,
    RulePack,
};

#[test]
fn contract_report_derives_stats_from_findings() {
    let mut first = ContractFinding::new(
        "REST-R001",
        "rest_docs",
        FindingSeverity::Error,
        FindingMode::Deterministic,
        "Missing endpoint purpose",
        "The endpoint is missing a purpose description.",
    );
    first
        .advisory_role_ids
        .push("rest_contract_auditor".to_string());

    let second = ContractFinding::new(
        "AUDIT-R003",
        "multi_role_audit",
        FindingSeverity::Warning,
        FindingMode::Advisory,
        "Runtime drift warning",
        "The runtime trace suggests unstable behavior.",
    );

    let report = ContractReport::from_findings(
        "xiuxian-testing-contracts",
        ContractExecutionMode::Advisory,
        "2026-03-17T00:00:00Z",
        vec![first, second],
    );

    assert_eq!(report.stats.total, 2);
    assert_eq!(
        report.stats,
        ContractStats {
            total: 2,
            info: 0,
            warning: 1,
            error: 1,
            critical: 0,
            deterministic: 1,
            advisory: 1,
            research: 0,
        }
    );
}

#[test]
fn contract_suite_registers_rule_packs() {
    let mut suite = ContractSuite::new("contracts", "v1");
    suite.register_rule_pack(Box::new(NoopRulePack));

    let descriptor = NoopRulePack.descriptor();

    assert_eq!(suite.id(), "contracts");
    assert_eq!(suite.version(), "v1");
    assert_eq!(suite.rule_pack_count(), 1);
    assert_eq!(descriptor.id, "noop");
}

#[test]
fn advisory_policy_and_role_finding_capture_runtime_metadata() {
    let policy = AdvisoryAuditPolicy {
        enabled: true,
        requested_roles: vec![
            "rest_contract_auditor".to_string(),
            "runtime_trace_reviewer".to_string(),
        ],
        min_severity: FindingSeverity::Warning,
    };

    let mut finding = RoleAuditFinding::new(
        "runtime_trace_reviewer",
        FindingSeverity::Warning,
        "The audit stream drifted before the endpoint contract stabilized.",
    )
    .with_trace_id("trace-123");
    finding.push_message_evidence("coherence dropped below the configured threshold");

    assert!(policy.enabled);
    assert_eq!(policy.requested_roles.len(), 2);
    assert_eq!(policy.min_severity, FindingSeverity::Warning);
    assert_eq!(finding.trace_id.as_deref(), Some("trace-123"));
    assert_eq!(finding.evidence.len(), 1);
}

#[test]
fn collection_context_defaults_to_empty_labels() {
    let context = CollectionContext::default();

    assert!(context.suite_id.is_empty());
    assert!(context.labels.is_empty());
    assert!(context.crate_name.is_none());
}
