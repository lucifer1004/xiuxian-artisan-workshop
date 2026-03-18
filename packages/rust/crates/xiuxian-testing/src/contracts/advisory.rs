//! Advisory audit interfaces for role-attributed contract review.

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::model::{
    CollectedArtifacts, ContractFinding, EvidenceKind, FindingConfidence, FindingEvidence,
    FindingExamples, FindingSeverity,
};

/// Policy that decides whether a rule pack should trigger advisory execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryAuditPolicy {
    /// Whether advisory execution is enabled.
    pub enabled: bool,
    /// Requested role identifiers for the advisory pass.
    pub requested_roles: Vec<String>,
    /// Minimum severity that should trigger the advisory pass.
    pub min_severity: FindingSeverity,
}

impl Default for AdvisoryAuditPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            requested_roles: Vec::new(),
            min_severity: FindingSeverity::Warning,
        }
    }
}

/// Request sent to an advisory audit executor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisoryAuditRequest {
    /// Stable suite identifier.
    pub suite_id: String,
    /// Rule-pack identifier.
    pub pack_id: String,
    /// Baseline findings from deterministic evaluation.
    pub findings: Vec<ContractFinding>,
    /// Collected artifacts available to the advisory lane.
    pub artifacts: CollectedArtifacts,
    /// Requested role identifiers.
    pub requested_roles: Vec<String>,
}

/// One role-attributed advisory finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleAuditFinding {
    /// Role identifier that produced the critique.
    pub role_id: String,
    /// Optional related deterministic rule identifier.
    pub rule_id: Option<String>,
    /// Assigned severity.
    pub severity: FindingSeverity,
    /// Confidence assigned by the advisory executor.
    pub confidence: FindingConfidence,
    /// Human-readable summary.
    pub summary: String,
    /// Why the critique matters.
    pub why_it_matters: String,
    /// Actionable remediation guidance.
    pub remediation: String,
    /// Evidence collected by the advisory lane.
    pub evidence: Vec<FindingEvidence>,
    /// Optional attached trace identifier.
    pub trace_id: Option<String>,
    /// Example snippets or cases attached by the role.
    pub examples: FindingExamples,
    /// Additional labels for downstream grouping.
    pub labels: BTreeMap<String, String>,
}

impl RoleAuditFinding {
    /// Create a minimal advisory finding.
    #[must_use]
    pub fn new(
        role_id: impl Into<String>,
        severity: FindingSeverity,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            role_id: role_id.into(),
            rule_id: None,
            severity,
            confidence: FindingConfidence::Medium,
            summary: summary.into(),
            why_it_matters: String::new(),
            remediation: String::new(),
            evidence: Vec::new(),
            trace_id: None,
            examples: FindingExamples::default(),
            labels: BTreeMap::new(),
        }
    }

    /// Attach a trace identifier to the finding.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Append one evidence message using a derived-invariant evidence kind.
    pub fn push_message_evidence(&mut self, message: impl Into<String>) {
        self.evidence.push(FindingEvidence {
            kind: EvidenceKind::DerivedInvariant,
            path: None,
            locator: None,
            message: message.into(),
        });
    }
}

/// Async executor for multi-role advisory audits.
#[async_trait]
pub trait AdvisoryAuditExecutor: Send + Sync {
    /// Execute one advisory audit request.
    ///
    /// # Errors
    ///
    /// Returns an error when advisory execution fails.
    async fn run(&self, request: AdvisoryAuditRequest) -> Result<Vec<RoleAuditFinding>>;
}

/// Minimal executor that emits no advisory findings.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAdvisoryAuditExecutor;

#[async_trait]
impl AdvisoryAuditExecutor for NoopAdvisoryAuditExecutor {
    async fn run(&self, _request: AdvisoryAuditRequest) -> Result<Vec<RoleAuditFinding>> {
        Ok(Vec::new())
    }
}
