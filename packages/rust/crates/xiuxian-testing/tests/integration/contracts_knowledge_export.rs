//! Focused integration coverage for contract knowledge export.

use std::path::PathBuf;

use xiuxian_testing::{
    ContractExecutionMode, ContractFinding, ContractKnowledgeBatch, ContractKnowledgeDecision,
    ContractKnowledgeEnvelope, ContractReport, EvidenceKind, FindingConfidence, FindingEvidence,
    FindingMode, FindingSeverity,
};

#[test]
fn knowledge_decision_follows_export_policy() {
    assert_eq!(
        ContractKnowledgeDecision::from_severity(FindingSeverity::Info),
        ContractKnowledgeDecision::Pass
    );
    assert_eq!(
        ContractKnowledgeDecision::from_severity(FindingSeverity::Warning),
        ContractKnowledgeDecision::Warn
    );
    assert_eq!(
        ContractKnowledgeDecision::from_severity(FindingSeverity::Error),
        ContractKnowledgeDecision::Fail
    );
    assert_eq!(
        ContractKnowledgeDecision::from_severity(FindingSeverity::Critical),
        ContractKnowledgeDecision::Fail
    );
}

#[test]
fn knowledge_envelope_preserves_contract_fields_and_provenance() {
    let mut finding = ContractFinding::new(
        "REST-R003",
        "rest_docs",
        FindingSeverity::Error,
        FindingMode::Deterministic,
        "Incomplete response documentation",
        "The endpoint is missing response descriptions.",
    );
    finding.confidence = FindingConfidence::Medium;
    finding.why_it_matters = "Clients need explicit success and error coverage.".to_string();
    finding.remediation = "Document both success and error responses.".to_string();
    finding
        .examples
        .good
        .push("Document `200` and `400` with short response descriptions.".to_string());
    finding
        .examples
        .bad
        .push("Expose statuses without any descriptions.".to_string());
    finding
        .advisory_role_ids
        .push("rest_contract_auditor".to_string());
    finding.trace_ids.push("trace-42".to_string());
    finding
        .labels
        .insert("domain".to_string(), "rest".to_string());
    finding
        .labels
        .insert("path".to_string(), "/api/search".to_string());
    finding.evidence.push(FindingEvidence {
        kind: EvidenceKind::OpenApiNode,
        path: Some(PathBuf::from(
            "packages/rust/crates/xiuxian-wendao/openapi.json",
        )),
        locator: Some("/paths/~1api~1search/get/responses/500".to_string()),
        message: "Error response `500` is missing a non-empty description.".to_string(),
    });

    let envelope = ContractKnowledgeEnvelope::from_finding(
        "wendao-contracts",
        "2026-03-17T00:00:00Z",
        &finding,
    );

    assert_eq!(envelope.suite_id, "wendao-contracts");
    assert_eq!(envelope.rule_id, "REST-R003");
    assert_eq!(envelope.pack_id, "rest_docs");
    assert_eq!(envelope.domain, "rest");
    assert_eq!(envelope.decision, ContractKnowledgeDecision::Fail);
    assert_eq!(envelope.confidence, FindingConfidence::Medium);
    assert_eq!(
        envelope.source_path,
        Some(PathBuf::from(
            "packages/rust/crates/xiuxian-wendao/openapi.json"
        ))
    );
    assert_eq!(
        envelope.evidence_excerpt.as_deref(),
        Some("Error response `500` is missing a non-empty description.")
    );
    assert_eq!(
        envelope.good_example.as_deref(),
        Some("Document `200` and `400` with short response descriptions.")
    );
    assert_eq!(
        envelope.bad_example.as_deref(),
        Some("Expose statuses without any descriptions.")
    );
    assert!(
        envelope
            .entry_id
            .contains("wendao-contracts::rest_docs::REST-R003")
    );
    assert!(envelope.tags.iter().any(|tag| tag == "domain:rest"));
    assert!(envelope.tags.iter().any(|tag| tag == "path:/api/search"));
    assert!(
        envelope
            .content
            .contains("Summary: The endpoint is missing response descriptions.")
    );
    assert!(
        envelope
            .content
            .contains("Why it matters: Clients need explicit success and error coverage.")
    );
    assert!(
        envelope
            .metadata
            .get("decision")
            .is_some_and(|value| value == "fail")
    );
}

#[test]
fn knowledge_batch_exports_all_report_findings() {
    let mut first = ContractFinding::new(
        "REST-R001",
        "rest_docs",
        FindingSeverity::Error,
        FindingMode::Deterministic,
        "Missing endpoint purpose",
        "The endpoint is missing a purpose description.",
    );
    first.why_it_matters = "A stable purpose statement keeps external callers aligned.".to_string();
    first.remediation = "Add a `summary` or `description`.".to_string();
    first
        .labels
        .insert("domain".to_string(), "rest".to_string());

    let mut second = ContractFinding::new(
        "AUDIT-R003",
        "multi_role_audit",
        FindingSeverity::Warning,
        FindingMode::Advisory,
        "Runtime drift warning",
        "The runtime trace suggests unstable behavior.",
    );
    second.why_it_matters = "Runtime drift should be visible even before hard failure.".to_string();
    second.remediation =
        "Review the trace and compare it to deterministic contract evidence.".to_string();
    second
        .labels
        .insert("domain".to_string(), "audit".to_string());

    let report = ContractReport::from_findings(
        "xiuxian-testing-contracts",
        ContractExecutionMode::Advisory,
        "2026-03-17T00:00:00Z",
        vec![first, second],
    );

    let batch = ContractKnowledgeBatch::from_report(&report);

    assert_eq!(batch.suite_id, "xiuxian-testing-contracts");
    assert_eq!(batch.generated_at, "2026-03-17T00:00:00Z");
    assert_eq!(batch.entries.len(), 2);
    assert_eq!(batch.entries[0].decision, ContractKnowledgeDecision::Fail);
    assert_eq!(batch.entries[1].decision, ContractKnowledgeDecision::Warn);
    assert_eq!(batch.entries[1].domain, "audit");
}

#[test]
fn knowledge_batch_uses_unique_entry_ids_across_modes_for_the_same_rule() {
    let mut deterministic = ContractFinding::new(
        "REST-R001",
        "rest_docs",
        FindingSeverity::Error,
        FindingMode::Deterministic,
        "Missing endpoint purpose",
        "The endpoint is missing a purpose description.",
    );
    deterministic.evidence.push(FindingEvidence {
        kind: EvidenceKind::OpenApiNode,
        path: Some(PathBuf::from("openapi.yaml")),
        locator: None,
        message: "GET /api/search is missing summary text.".to_string(),
    });

    let mut advisory = ContractFinding::new(
        "REST-R001",
        "rest_docs",
        FindingSeverity::Error,
        FindingMode::Advisory,
        "Strict Teacher prepared advisory review",
        "Strict Teacher prepared advisory review for one deterministic finding.",
    );
    advisory
        .advisory_role_ids
        .push("strict_teacher".to_string());
    advisory.evidence.push(FindingEvidence {
        kind: EvidenceKind::OpenApiNode,
        path: Some(PathBuf::from("openapi.yaml")),
        locator: None,
        message: "Advisory review references the same endpoint.".to_string(),
    });

    let report = ContractReport::from_findings(
        "contracts",
        ContractExecutionMode::Advisory,
        "2026-03-17T00:00:00Z",
        vec![deterministic, advisory],
    );

    let batch = ContractKnowledgeBatch::from_report(&report);

    assert_eq!(batch.entries.len(), 2);
    assert_ne!(batch.entries[0].entry_id, batch.entries[1].entry_id);
    assert!(batch.entries[0].entry_id.contains("::deterministic::"));
    assert!(batch.entries[1].entry_id.contains("::advisory::"));
    assert!(batch.entries[1].entry_id.contains("::role:strict_teacher"));
}
