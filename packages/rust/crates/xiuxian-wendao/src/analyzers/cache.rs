//! In-memory and Valkey-backed analysis cache for repository intelligence.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::git::checkout::{LocalCheckoutMetadata, ResolvedRepositorySource};

/// Cache key for repository analysis results.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositoryAnalysisCacheKey {
    /// Repository identifier.
    pub repo_id: String,
    /// Root path of the checkout.
    pub checkout_root: String,
    /// Revision of the checkout.
    pub checkout_revision: Option<String>,
    /// Revision of the mirror.
    pub mirror_revision: Option<String>,
    /// Revision being tracked.
    pub tracking_revision: Option<String>,
    /// Sorted list of plugin identifiers used.
    pub plugin_ids: Vec<String>,
}

type RepositoryAnalysisCache = BTreeMap<RepositoryAnalysisCacheKey, RepositoryAnalysisOutput>;

static REPOSITORY_ANALYSIS_CACHE: OnceLock<Mutex<RepositoryAnalysisCache>> = OnceLock::new();

fn repository_analysis_cache() -> &'static Mutex<RepositoryAnalysisCache> {
    REPOSITORY_ANALYSIS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Builds a cache key from repository configuration and resolved source.
#[must_use]
pub fn build_repository_analysis_cache_key(
    repository: &RegisteredRepository,
    source: &ResolvedRepositorySource,
    metadata: Option<&LocalCheckoutMetadata>,
) -> RepositoryAnalysisCacheKey {
    let mut plugin_ids = repository
        .plugins
        .iter()
        .map(|plugin| plugin.id().to_string())
        .collect::<Vec<_>>();
    plugin_ids.sort_unstable();
    plugin_ids.dedup();

    RepositoryAnalysisCacheKey {
        repo_id: repository.id.clone(),
        checkout_root: source.checkout_root.display().to_string(),
        checkout_revision: metadata.and_then(|item| item.revision.clone()),
        mirror_revision: source.mirror_revision.clone(),
        tracking_revision: source.tracking_revision.clone(),
        plugin_ids,
    }
}

/// Loads a cached analysis result if available.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned.
pub fn load_cached_repository_analysis(
    key: &RepositoryAnalysisCacheKey,
) -> Result<Option<RepositoryAnalysisOutput>, RepoIntelligenceError> {
    repository_analysis_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository analysis cache lock is poisoned".to_string(),
        })
        .map(|cache| cache.get(key).cloned())
}

/// Stores an analysis result in the cache.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned.
pub fn store_cached_repository_analysis(
    key: RepositoryAnalysisCacheKey,
    output: &RepositoryAnalysisOutput,
) -> Result<(), RepoIntelligenceError> {
    repository_analysis_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository analysis cache lock is poisoned".to_string(),
        })
        .map(|mut cache| {
            cache.insert(key, output.clone());
        })
}

/// Valkey-backed analysis cache placeholder.
pub struct ValkeyAnalysisCache {
    // Current MVP focuses on memory-first with OnceLock,
    // real Valkey integration would happen here.
}

impl ValkeyAnalysisCache {
    /// Creates a new Valkey cache client if configured.
    ///
    /// # Errors
    ///
    /// This placeholder implementation does not currently fail.
    #[allow(clippy::unnecessary_wraps)]
    pub fn new() -> Result<Option<Self>, RepoIntelligenceError> {
        Ok(None)
    }

    /// Retrieves a cached analysis result.
    ///
    /// # Errors
    ///
    /// This placeholder implementation does not currently fail.
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    pub fn get(
        &self,
        _repository: &RegisteredRepository,
        _revision: &str,
    ) -> Result<Option<RepositoryAnalysisOutput>, RepoIntelligenceError> {
        Ok(None)
    }

    /// Stores an analysis result in the cache.
    ///
    /// # Errors
    ///
    /// This placeholder implementation does not currently fail.
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    pub fn set(
        &self,
        _repository: &RegisteredRepository,
        _revision: &str,
        _output: RepositoryAnalysisOutput,
    ) -> Result<(), RepoIntelligenceError> {
        Ok(())
    }
}
