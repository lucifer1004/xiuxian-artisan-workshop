//! Shared search infrastructure and primitives for Wendao.

mod attachment;
mod cache;
mod coordinator;
mod corpus;
/// Shared lexical fuzzy-search utilities.
pub mod fuzzy;
mod knowledge_section;
mod local_symbol;
mod manifest;
mod project_fingerprint;
/// Shared query-language adapters that sit above the Wendao search runtime.
pub mod queries;
mod ranking;
mod reference_occurrence;
mod repo_content_chunk;
mod repo_entity;
mod repo_publication_parquet;
/// Shared repo-search execution seams above the search runtime.
pub(crate) mod repo_search;
mod repo_staging;
mod service;
mod staged_mutation;
mod status;
/// Shared Tantivy-backed search primitives.
pub mod tantivy;

pub(crate) use attachment::AttachmentSearchError;
#[cfg(test)]
pub(crate) use cache::SearchPlaneCache;
pub(crate) use cache::SearchPlaneCacheTtl;
pub(crate) use cache::resolve_search_plane_cache_connection_target;
pub use coordinator::{BeginBuildDecision, SearchBuildLease, SearchPlaneCoordinator};
pub use corpus::SearchCorpusKind;
pub use fuzzy::{
    FuzzyMatch, FuzzyMatcher, FuzzyScore, FuzzySearchOptions, LexicalMatcher, edit_distance,
    levenshtein_distance, normalized_score, passes_prefix_requirement, shared_prefix_len,
};
pub(crate) use knowledge_section::KnowledgeSectionSearchError;
pub(crate) use local_symbol::LocalSymbolSearchError;
pub(crate) use manifest::SearchRepoPublicationInput;
pub use manifest::{
    SearchFileFingerprint, SearchManifestKeyspace, SearchManifestRecord,
    SearchPublicationStorageFormat, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
    SearchRepoPublicationRecord, SearchRepoRuntimeRecord,
};
#[allow(unused_imports)]
pub(crate) use project_fingerprint::{
    ProjectScannedFile, fingerprint_note_projects, fingerprint_source_projects,
    fingerprint_symbol_projects, scan_note_project_files, scan_source_project_files,
    scan_symbol_project_files,
};
pub(crate) use reference_occurrence::ReferenceOccurrenceSearchError;
#[cfg(test)]
pub(crate) use reference_occurrence::{reference_occurrence_batches, reference_occurrence_schema};
pub(crate) use repo_content_chunk::RepoContentChunkSearchFilters;
#[cfg(test)]
pub(crate) use repo_entity::publish_repo_entities;
pub(crate) use repo_entity::{
    search_repo_entity_example_results, search_repo_entity_import_results,
    search_repo_entity_module_results, search_repo_entity_symbol_results,
};
pub(crate) use repo_staging::{
    RepoStagedMutationAction, RepoStagedMutationPlan, plan_repo_staged_mutation,
};
pub(crate) use service::RepoSearchAvailability;
pub(crate) use service::RepoSearchPublicationState;
pub(crate) use service::RepoSearchQueryCacheKeyInput;
pub use service::SearchPlaneService;
pub(crate) use staged_mutation::delete_paths_from_table;
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
