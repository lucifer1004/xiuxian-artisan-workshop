//! Wendao-ready export surface for contract findings and reports.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::model::{
    ContractFinding, ContractReport, EvidenceKind, FindingConfidence, FindingSeverity,
};

/// Decision label exported alongside one contract finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractKnowledgeDecision {
    /// The finding is informational and should not block the flow.
    Pass,
    /// The finding should be surfaced as advisory guidance.
    Warn,
    /// The finding should be treated as a failing contract signal.
    Fail,
}

impl ContractKnowledgeDecision {
    /// Map one finding severity to an export decision.
    #[must_use]
    pub const fn from_severity(severity: FindingSeverity) -> Self {
        match severity {
            FindingSeverity::Info => Self::Pass,
            FindingSeverity::Warning => Self::Warn,
            FindingSeverity::Error | FindingSeverity::Critical => Self::Fail,
        }
    }

    /// Return the canonical serialized label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }
}

/// One ingestion-ready knowledge envelope derived from a contract finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractKnowledgeEnvelope {
    /// Stable export identifier for downstream storage.
    pub entry_id: String,
    /// Contract suite identifier that produced the finding.
    pub suite_id: String,
    /// Generation timestamp inherited from the parent report.
    pub generated_at: String,
    /// Stable rule identifier.
    pub rule_id: String,
    /// Rule-pack identifier.
    pub pack_id: String,
    /// Logical domain for grouping and filtering.
    pub domain: String,
    /// Exported severity.
    pub severity: FindingSeverity,
    /// Exported decision.
    pub decision: ContractKnowledgeDecision,
    /// Confidence attached to the original finding.
    pub confidence: FindingConfidence,
    /// Human-readable title suitable for `KnowledgeEntry.title`.
    pub title: String,
    /// Human-readable body suitable for `KnowledgeEntry.content`.
    pub content: String,
    /// Short summary from the original finding.
    pub summary: String,
    /// First evidence excerpt for fast inspection.
    pub evidence_excerpt: Option<String>,
    /// Why the finding matters.
    pub why_it_matters: String,
    /// Actionable remediation guidance.
    pub remediation: String,
    /// First positive example, if any.
    pub good_example: Option<String>,
    /// First negative example, if any.
    pub bad_example: Option<String>,
    /// First source path discovered from the finding evidence.
    pub source_path: Option<PathBuf>,
    /// Search-friendly tags for downstream indexing.
    pub tags: Vec<String>,
    /// Structured metadata for later adaptation into Wendao-specific types.
    pub metadata: BTreeMap<String, Value>,
}

impl ContractKnowledgeEnvelope {
    /// Create one Wendao-ready knowledge envelope from a contract finding.
    #[must_use]
    pub fn from_finding(
        suite_id: impl Into<String>,
        generated_at: impl Into<String>,
        finding: &ContractFinding,
    ) -> Self {
        let suite_id = suite_id.into();
        let generated_at = generated_at.into();
        let domain = finding
            .labels
            .get("domain")
            .cloned()
            .unwrap_or_else(|| finding.pack_id.clone());
        let evidence_excerpt = finding
            .evidence
            .first()
            .map(|evidence| evidence.message.clone());
        let source_path = first_source_path(finding);
        let good_example = finding.examples.good.first().cloned();
        let bad_example = finding.examples.bad.first().cloned();
        let decision = ContractKnowledgeDecision::from_severity(finding.severity);
        let entry_id = build_entry_id(&suite_id, finding);
        let title = format!("[{}] {}", finding.rule_id, finding.title);
        let content = render_content(finding, evidence_excerpt.as_deref());
        let tags = build_tags(&domain, finding);
        let metadata = build_metadata(&suite_id, &generated_at, &domain, decision, finding);

        Self {
            entry_id,
            suite_id,
            generated_at,
            rule_id: finding.rule_id.clone(),
            pack_id: finding.pack_id.clone(),
            domain,
            severity: finding.severity,
            decision,
            confidence: finding.confidence,
            title,
            content,
            summary: finding.summary.clone(),
            evidence_excerpt,
            why_it_matters: finding.why_it_matters.clone(),
            remediation: finding.remediation.clone(),
            good_example,
            bad_example,
            source_path,
            tags,
            metadata,
        }
    }
}

/// Batch export for one contract report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractKnowledgeBatch {
    /// Contract suite identifier.
    pub suite_id: String,
    /// Timestamp inherited from the parent report.
    pub generated_at: String,
    /// Exported knowledge envelopes.
    pub entries: Vec<ContractKnowledgeEnvelope>,
}

impl ContractKnowledgeBatch {
    /// Transform one contract report into a batch of Wendao-ready knowledge envelopes.
    #[must_use]
    pub fn from_report(report: &ContractReport) -> Self {
        let entries = report
            .findings
            .iter()
            .map(|finding| {
                ContractKnowledgeEnvelope::from_finding(
                    report.suite_id.clone(),
                    report.generated_at.clone(),
                    finding,
                )
            })
            .collect();

        Self {
            suite_id: report.suite_id.clone(),
            generated_at: report.generated_at.clone(),
            entries,
        }
    }
}

fn build_entry_id(suite_id: &str, finding: &ContractFinding) -> String {
    let path_fragment = finding
        .labels
        .get("path")
        .cloned()
        .or_else(|| {
            first_source_path(finding).map(|path| {
                path.to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "::")
            })
        })
        .unwrap_or_else(|| "global".to_string());
    let mode_fragment = finding_mode_label(finding.mode);
    let advisory_role_fragment = advisory_role_fragment(finding);
    format!(
        "{suite_id}::{}::{}::{mode_fragment}::{path_fragment}{advisory_role_fragment}",
        finding.pack_id, finding.rule_id,
    )
}

fn render_content(finding: &ContractFinding, evidence_excerpt: Option<&str>) -> String {
    let mut sections = vec![
        format!("Summary: {}", finding.summary),
        format!("Why it matters: {}", finding.why_it_matters),
        format!("Remediation: {}", finding.remediation),
    ];

    if let Some(evidence_excerpt) = evidence_excerpt {
        sections.push(format!("Evidence: {evidence_excerpt}"));
    }
    if let Some(example) = finding.examples.good.first() {
        sections.push(format!("Good example: {example}"));
    }
    if let Some(example) = finding.examples.bad.first() {
        sections.push(format!("Bad example: {example}"));
    }

    sections.join("\n")
}

fn build_tags(domain: &str, finding: &ContractFinding) -> Vec<String> {
    let mut tags = vec![
        "contract_finding".to_string(),
        format!("pack:{}", finding.pack_id),
        format!("rule:{}", finding.rule_id),
        format!("severity:{}", finding_severity_label(finding.severity)),
        format!("mode:{}", finding_mode_label(finding.mode)),
        format!("domain:{domain}"),
    ];

    if let Some(path) = finding.labels.get("path") {
        tags.push(format!("path:{path}"));
    }

    tags.sort();
    tags.dedup();
    tags
}

fn build_metadata(
    suite_id: &str,
    generated_at: &str,
    domain: &str,
    decision: ContractKnowledgeDecision,
    finding: &ContractFinding,
) -> BTreeMap<String, Value> {
    let mut metadata = BTreeMap::new();
    metadata.insert("suite_id".to_string(), json!(suite_id));
    metadata.insert("generated_at".to_string(), json!(generated_at));
    metadata.insert("domain".to_string(), json!(domain));
    metadata.insert("decision".to_string(), json!(decision.as_str()));
    metadata.insert(
        "confidence".to_string(),
        json!(finding_confidence_label(finding.confidence)),
    );
    metadata.insert(
        "advisory_role_ids".to_string(),
        json!(finding.advisory_role_ids),
    );
    metadata.insert("trace_ids".to_string(), json!(finding.trace_ids));
    metadata.insert("labels".to_string(), json!(finding.labels));
    metadata.insert(
        "evidence_kinds".to_string(),
        json!(
            finding
                .evidence
                .iter()
                .map(|evidence| evidence_kind_label(evidence.kind))
                .collect::<Vec<_>>()
        ),
    );
    metadata
}

fn first_source_path(finding: &ContractFinding) -> Option<PathBuf> {
    finding
        .evidence
        .iter()
        .find_map(|evidence| evidence.path.clone())
        .or_else(|| finding.labels.get("source_path").map(PathBuf::from))
}

fn finding_severity_label(severity: FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Info => "info",
        FindingSeverity::Warning => "warning",
        FindingSeverity::Error => "error",
        FindingSeverity::Critical => "critical",
    }
}

fn finding_mode_label(mode: super::model::FindingMode) -> &'static str {
    match mode {
        super::model::FindingMode::Deterministic => "deterministic",
        super::model::FindingMode::Advisory => "advisory",
        super::model::FindingMode::Research => "research",
    }
}

fn advisory_role_fragment(finding: &ContractFinding) -> String {
    if finding.mode != super::model::FindingMode::Advisory {
        return String::new();
    }

    finding
        .advisory_role_ids
        .first()
        .cloned()
        .or_else(|| finding.labels.get("role_id").cloned())
        .map(|role_id| format!("::role:{}", normalized_fragment(&role_id)))
        .unwrap_or_default()
}

fn normalized_fragment(fragment: &str) -> String {
    let mut normalized = String::with_capacity(fragment.len());
    for character in fragment.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
        } else {
            normalized.push('_');
        }
    }
    normalized
}

fn finding_confidence_label(confidence: FindingConfidence) -> &'static str {
    match confidence {
        FindingConfidence::High => "high",
        FindingConfidence::Medium => "medium",
        FindingConfidence::Low => "low",
    }
}

fn evidence_kind_label(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::SourceSpan => "source_span",
        EvidenceKind::OpenApiNode => "openapi_node",
        EvidenceKind::DocSection => "doc_section",
        EvidenceKind::RuntimeTrace => "runtime_trace",
        EvidenceKind::ScenarioSnapshot => "scenario_snapshot",
        EvidenceKind::DerivedInvariant => "derived_invariant",
    }
}
