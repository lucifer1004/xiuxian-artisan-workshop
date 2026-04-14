//! High-level repository intelligence service orchestration.

mod analysis;
mod bootstrap;
mod cached;
pub(crate) mod helpers;
#[cfg(feature = "zhenfa-router")]
mod incremental;
mod julia_transport;
mod merge;
mod projection;
mod registry;
mod relation_dedupe;
mod search;
mod sync;

#[cfg(feature = "studio")]
pub(crate) use analysis::analyze_registered_repository_target_file_with_registry;
pub use analysis::{
    analyze_registered_repository, analyze_registered_repository_with_registry,
    analyze_repository_from_config, analyze_repository_from_config_with_registry,
};
pub use bootstrap::bootstrap_builtin_registry;
pub use cached::analyze_registered_repository_cached_with_registry;
#[cfg(feature = "studio")]
pub(crate) use cached::{
    CachedRepositoryAnalysis, analyze_registered_repository_cached_bundle_with_registry,
};
pub use helpers::relation_kind_label;
#[cfg(feature = "zhenfa-router")]
pub(crate) use incremental::{
    IncrementalApplyContext, analyze_changed_files, apply_incremental_plugin_outputs,
};
pub use julia_transport::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_EMBEDDING_COLUMN,
    JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_QUERY_EMBEDDING_COLUMN,
    JULIA_ARROW_TRACE_ID_COLUMN, JULIA_ARROW_VECTOR_SCORE_COLUMN, julia_arrow_request_schema,
    julia_arrow_response_schema,
};

#[cfg(feature = "studio")]
pub(crate) use projection::build_repo_projected_page_search_with_artifacts;
pub use projection::*;
#[cfg(any(test, feature = "zhenfa-router"))]
pub(crate) use projection::{DocsToolRuntime, DocsToolRuntimeHandle};
pub use registry::load_registered_repository;
#[cfg(feature = "studio")]
pub(crate) use search::ExampleSearchMetadata;
#[cfg(feature = "search-runtime")]
pub(crate) use search::canonical_import_query_text;
pub use search::*;
#[cfg(feature = "studio")]
pub(crate) use search::{
    RepoAnalysisFallbackContract, example_fallback_contract, import_fallback_contract,
    module_fallback_contract, repository_search_artifacts, symbol_fallback_contract,
};
pub use sync::*;
#[cfg(test)]
#[path = "../../../tests/unit/analyzers/service/mod.rs"]
mod tests;
