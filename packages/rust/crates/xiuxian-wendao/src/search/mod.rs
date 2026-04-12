//! Shared search infrastructure and primitives for Wendao.

#[cfg(feature = "search-runtime")]
mod attachment;
#[cfg(feature = "search-runtime")]
mod cache;
#[cfg(feature = "search-runtime")]
mod coordinator;
#[cfg(feature = "search-runtime")]
mod corpus;
/// Shared lexical fuzzy-search utilities.
pub mod fuzzy;
#[cfg(feature = "search-runtime")]
mod knowledge_section;
#[cfg(feature = "search-runtime")]
mod local_publication_parquet;
#[cfg(feature = "search-runtime")]
mod local_symbol;
#[cfg(feature = "search-runtime")]
mod manifest;
#[cfg(feature = "search-runtime")]
mod project_fingerprint;
/// Shared query-language adapters that sit above the Wendao search runtime.
pub mod queries;
#[cfg(feature = "search-runtime")]
mod ranking;
#[cfg(feature = "search-runtime")]
mod reference_occurrence;
#[cfg(feature = "search-runtime")]
mod repo_content_chunk;
#[cfg(feature = "search-runtime")]
mod repo_entity;
#[cfg(feature = "search-runtime")]
mod repo_publication_parquet;
/// Shared repo-search execution seams above the search runtime.
#[cfg(feature = "search-runtime")]
pub(crate) mod repo_search;
#[cfg(feature = "search-runtime")]
mod repo_staging;
#[cfg(feature = "search-runtime")]
mod service;
#[cfg(feature = "search-runtime")]
mod status;
/// Shared Tantivy-backed search primitives.
pub mod tantivy;

#[cfg(feature = "search-runtime")]
pub(crate) use attachment::AttachmentSearchError;
#[cfg(all(test, feature = "search-runtime"))]
pub(crate) use cache::SearchPlaneCache;
#[cfg(feature = "search-runtime")]
pub(crate) use cache::SearchPlaneCacheTtl;
#[cfg(feature = "search-runtime")]
pub(crate) use cache::resolve_search_plane_cache_connection_target;
#[cfg(feature = "search-runtime")]
pub use coordinator::{BeginBuildDecision, SearchBuildLease, SearchPlaneCoordinator};
#[cfg(feature = "search-runtime")]
pub use corpus::SearchCorpusKind;
pub use fuzzy::{
    FuzzyMatch, FuzzyMatcher, FuzzyScore, FuzzySearchOptions, LexicalMatcher, edit_distance,
    levenshtein_distance, normalized_score, passes_prefix_requirement, shared_prefix_len,
};
#[cfg(feature = "search-runtime")]
pub(crate) use knowledge_section::KnowledgeSectionSearchError;
#[cfg(feature = "search-runtime")]
pub(crate) use local_symbol::LocalSymbolSearchError;
#[cfg(feature = "search-runtime")]
pub(crate) use manifest::SearchRepoPublicationInput;
#[cfg(feature = "search-runtime")]
pub use manifest::{
    SearchFileFingerprint, SearchManifestKeyspace, SearchManifestRecord,
    SearchPublicationStorageFormat, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
    SearchRepoPublicationRecord, SearchRepoRuntimeRecord,
};
#[cfg(feature = "search-runtime")]
#[allow(unused_imports)]
pub(crate) use project_fingerprint::{
    ProjectScannedFile, fingerprint_note_projects, fingerprint_source_projects,
    fingerprint_symbol_projects, scan_note_project_files, scan_source_project_files,
    scan_symbol_project_files,
};
#[cfg(feature = "search-runtime")]
pub(crate) use reference_occurrence::ReferenceOccurrenceSearchError;
#[cfg(all(test, feature = "search-runtime"))]
pub(crate) use reference_occurrence::{reference_occurrence_batches, reference_occurrence_schema};
#[cfg(feature = "search-runtime")]
pub(crate) use repo_content_chunk::RepoContentChunkSearchFilters;
#[cfg(all(test, feature = "search-runtime"))]
pub(crate) use repo_entity::publish_repo_entities;
#[cfg(feature = "search-runtime")]
pub(crate) use repo_entity::{
    search_repo_entity_example_results, search_repo_entity_import_results,
    search_repo_entity_module_results, search_repo_entity_symbol_results,
};
#[cfg(feature = "search-runtime")]
pub(crate) use repo_staging::{
    RepoStagedMutationAction, RepoStagedMutationPlan, plan_repo_staged_mutation,
};
#[cfg(feature = "search-runtime")]
pub(crate) use service::RepoSearchAvailability;
#[cfg(feature = "search-runtime")]
pub(crate) use service::RepoSearchPublicationState;
#[cfg(feature = "search-runtime")]
pub(crate) use service::RepoSearchQueryCacheKeyInput;
#[cfg(feature = "search-runtime")]
pub use service::SearchPlaneService;
#[cfg(feature = "search-runtime")]
pub use status::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusIssueSummary,
    SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
    SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchMaintenancePolicy,
    SearchMaintenanceStatus, SearchPlanePhase, SearchPlaneStatusSnapshot, SearchQueryTelemetry,
    SearchQueryTelemetrySource, SearchRepoReadPressure,
};
pub use tantivy::{
    SearchDocument, SearchDocumentFields, SearchDocumentHit, SearchDocumentIndex,
    SearchDocumentMatchField, TantivyDocumentMatch, TantivyMatcher,
};
