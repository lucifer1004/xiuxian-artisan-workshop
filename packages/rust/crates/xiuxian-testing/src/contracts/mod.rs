//! Contract-kernel surface for `xiuxian-testing`.

mod advisory;
mod model;
mod rule_pack;

pub use advisory::{
    AdvisoryAuditExecutor, AdvisoryAuditPolicy, AdvisoryAuditRequest, NoopAdvisoryAuditExecutor,
    RoleAuditFinding,
};
pub use model::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, CollectionContext, ContractExecutionMode,
    ContractFinding, ContractReport, ContractStats, EvidenceKind, FindingConfidence,
    FindingEvidence, FindingExamples, FindingMode, FindingSeverity,
};
pub use rule_pack::{ContractSuite, NoopRulePack, RulePack, RulePackDescriptor};
