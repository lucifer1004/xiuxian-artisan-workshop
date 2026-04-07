//! In-memory and Valkey-backed analysis cache for repository intelligence.

mod analysis;
mod artifacts;
mod identity;
mod keys;
mod query;
mod valkey;

#[cfg(test)]
mod tests;

pub use analysis::{
    load_cached_repository_analysis, load_cached_repository_analysis_for_revision,
    store_cached_repository_analysis,
};
pub use artifacts::{
    RepositorySearchArtifacts, load_cached_repository_search_artifacts,
    store_cached_repository_search_artifacts,
};
pub(crate) use identity::{
    FingerprintMode, analysis_fingerprint_mode, change_affects_analysis_identity,
};
pub(crate) use keys::build_repository_analysis_cache_key;
pub use keys::{RepositoryAnalysisCacheKey, RepositorySearchQueryCacheKey};
pub use query::{load_cached_repository_search_result, store_cached_repository_search_result};
pub(crate) use valkey::ValkeyAnalysisCache;
