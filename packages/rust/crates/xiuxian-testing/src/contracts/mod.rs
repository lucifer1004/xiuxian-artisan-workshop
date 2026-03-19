//! Contract-kernel surface for `xiuxian-testing`.

mod advisory;
mod export;
mod model;
mod packs;
mod rule_pack;
mod runner;

pub use advisory::{
    AdvisoryAuditExecutor, AdvisoryAuditPolicy, AdvisoryAuditRequest, NoopAdvisoryAuditExecutor,
    RoleAuditFinding,
};
pub use export::{ContractKnowledgeBatch, ContractKnowledgeDecision, ContractKnowledgeEnvelope};
pub use model::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, CollectionContext, ContractExecutionMode,
    ContractFinding, ContractReport, ContractStats, EvidenceKind, FindingConfidence,
    FindingEvidence, FindingExamples, FindingMode, FindingSeverity,
};
pub use packs::{ModularityRulePack, RestDocsRulePack};
pub use rule_pack::{ContractSuite, NoopRulePack, RulePack, RulePackDescriptor};
pub use runner::{ContractRunConfig, ContractSuiteRunner};
