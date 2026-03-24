mod attachment;
mod cache;
mod coordinator;
mod corpus;
mod knowledge_section;
mod local_symbol;
mod manifest;
mod ranking;
mod reference_occurrence;
mod repo_content_chunk;
mod repo_entity;
mod service;
mod status;

pub(crate) use attachment::AttachmentSearchError;
pub(crate) use cache::SearchPlaneCacheTtl;
pub use coordinator::{BeginBuildDecision, SearchBuildLease, SearchPlaneCoordinator};
pub use corpus::SearchCorpusKind;
pub(crate) use knowledge_section::KnowledgeSectionSearchError;
pub(crate) use local_symbol::LocalSymbolSearchError;
pub(crate) use manifest::SearchRepoPublicationInput;
pub use manifest::{
    SearchFileFingerprint, SearchManifestKeyspace, SearchManifestRecord, SearchRepoCorpusRecord,
    SearchRepoCorpusSnapshotRecord, SearchRepoPublicationRecord, SearchRepoRuntimeRecord,
};
pub(crate) use reference_occurrence::ReferenceOccurrenceSearchError;
pub(crate) use service::RepoSearchAvailability;
pub(crate) use service::RepoSearchQueryCacheKeyInput;
pub use service::SearchPlaneService;
pub use status::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusIssueSummary,
    SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
    SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchMaintenancePolicy,
    SearchMaintenanceStatus, SearchPlanePhase, SearchPlaneStatusSnapshot, SearchQueryTelemetry,
    SearchQueryTelemetrySource,
};
