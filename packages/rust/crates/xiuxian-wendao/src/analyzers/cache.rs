//! In-memory and Valkey-backed analysis cache for repository intelligence.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::service::ExampleSearchMetadata;
use crate::analyzers::{ExampleRecord, ModuleRecord, ProjectedPageRecord, SymbolRecord};
use crate::git::checkout::{LocalCheckoutMetadata, ResolvedRepositorySource};
use crate::search::{FuzzySearchOptions, SearchDocumentIndex};

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

/// Immutable search artifacts derived from one cached repository analysis snapshot.
#[derive(Clone)]
pub struct RepositorySearchArtifacts {
    /// Shared Tantivy index for module search.
    pub(crate) module_index: SearchDocumentIndex,
    /// Shared Tantivy index for symbol search.
    pub(crate) symbol_index: SearchDocumentIndex,
    /// Shared Tantivy index for example search.
    pub(crate) example_index: SearchDocumentIndex,
    /// Shared Tantivy index for projected-page search.
    pub(crate) projected_page_index: SearchDocumentIndex,
    /// Stable module lookup by identifier.
    pub(crate) modules_by_id: BTreeMap<String, ModuleRecord>,
    /// Stable symbol lookup by identifier.
    pub(crate) symbols_by_id: BTreeMap<String, SymbolRecord>,
    /// Stable example lookup by identifier.
    pub(crate) examples_by_id: BTreeMap<String, ExampleRecord>,
    /// Precomputed example metadata reused by search ranking.
    pub(crate) example_metadata: BTreeMap<String, ExampleSearchMetadata>,
    /// Stable projected-page lookup by page identifier.
    pub(crate) projected_pages_by_id: HashMap<String, ProjectedPageRecord>,
    /// Materialized projected pages reused by heuristic and lexical fallback.
    pub(crate) projected_pages: Vec<ProjectedPageRecord>,
}

/// Cache key for final repo-search endpoint payloads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositorySearchQueryCacheKey {
    /// The underlying analysis cache identity.
    pub analysis_key: RepositoryAnalysisCacheKey,
    /// Stable endpoint identifier.
    pub endpoint: String,
    /// Raw query text.
    pub query: String,
    /// Optional endpoint-specific filter such as projected-page kind.
    pub filter: Option<String>,
    /// Maximum edit distance for the search profile.
    pub max_distance: u8,
    /// Required shared prefix length for the search profile.
    pub prefix_length: usize,
    /// Whether transpositions are allowed for the search profile.
    pub transposition: bool,
    /// Result limit.
    pub limit: usize,
}

impl RepositorySearchQueryCacheKey {
    /// Build one endpoint cache key from the shared analysis identity plus query settings.
    #[must_use]
    pub fn new(
        analysis_key: &RepositoryAnalysisCacheKey,
        endpoint: &str,
        query: &str,
        filter: Option<String>,
        options: FuzzySearchOptions,
        limit: usize,
    ) -> Self {
        Self {
            analysis_key: analysis_key.clone(),
            endpoint: endpoint.to_string(),
            query: query.to_string(),
            filter,
            max_distance: options.max_distance,
            prefix_length: options.prefix_length,
            transposition: options.transposition,
            limit,
        }
    }
}

type RepositoryAnalysisCache = BTreeMap<RepositoryAnalysisCacheKey, RepositoryAnalysisOutput>;
type RepositorySearchArtifactsCache =
    BTreeMap<RepositoryAnalysisCacheKey, Arc<RepositorySearchArtifacts>>;
type RepositorySearchQueryCache = BTreeMap<RepositorySearchQueryCacheKey, serde_json::Value>;

static REPOSITORY_ANALYSIS_CACHE: OnceLock<Mutex<RepositoryAnalysisCache>> = OnceLock::new();
static REPOSITORY_SEARCH_ARTIFACTS_CACHE: OnceLock<Mutex<RepositorySearchArtifactsCache>> =
    OnceLock::new();
static REPOSITORY_SEARCH_QUERY_CACHE: OnceLock<Mutex<RepositorySearchQueryCache>> = OnceLock::new();

fn repository_analysis_cache() -> &'static Mutex<RepositoryAnalysisCache> {
    REPOSITORY_ANALYSIS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn repository_search_query_cache() -> &'static Mutex<RepositorySearchQueryCache> {
    REPOSITORY_SEARCH_QUERY_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn repository_search_artifacts_cache() -> &'static Mutex<RepositorySearchArtifactsCache> {
    REPOSITORY_SEARCH_ARTIFACTS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
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

/// Loads cached repository search artifacts if available.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned.
pub fn load_cached_repository_search_artifacts(
    key: &RepositoryAnalysisCacheKey,
) -> Result<Option<Arc<RepositorySearchArtifacts>>, RepoIntelligenceError> {
    repository_search_artifacts_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository search artifacts cache lock is poisoned".to_string(),
        })
        .map(|cache| cache.get(key).cloned())
}

/// Stores repository search artifacts in the cache and returns the shared handle.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned.
pub fn store_cached_repository_search_artifacts(
    key: RepositoryAnalysisCacheKey,
    artifacts: RepositorySearchArtifacts,
) -> Result<Arc<RepositorySearchArtifacts>, RepoIntelligenceError> {
    let artifacts = Arc::new(artifacts);
    repository_search_artifacts_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository search artifacts cache lock is poisoned".to_string(),
        })
        .map(|mut cache| {
            cache.insert(key, Arc::clone(&artifacts));
            artifacts
        })
}

/// Loads a cached repo-search payload if available.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned or payload decoding fails.
pub fn load_cached_repository_search_result<T>(
    key: &RepositorySearchQueryCacheKey,
) -> Result<Option<T>, RepoIntelligenceError>
where
    T: serde::de::DeserializeOwned,
{
    repository_search_query_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository search query cache lock is poisoned".to_string(),
        })?
        .get(key)
        .cloned()
        .map(|value| {
            serde_json::from_value(value).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!("failed to decode cached repository search payload: {error}"),
            })
        })
        .transpose()
}

/// Stores a repo-search payload in the query-result cache.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned or payload serialization fails.
pub fn store_cached_repository_search_result<T>(
    key: RepositorySearchQueryCacheKey,
    value: &T,
) -> Result<(), RepoIntelligenceError>
where
    T: serde::Serialize,
{
    let encoded =
        serde_json::to_value(value).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!("failed to encode cached repository search payload: {error}"),
        })?;
    repository_search_query_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository search query cache lock is poisoned".to_string(),
        })
        .map(|mut cache| {
            cache.insert(key, encoded);
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use super::{
        RepositoryAnalysisCacheKey, RepositorySearchArtifacts, RepositorySearchQueryCacheKey,
        load_cached_repository_search_artifacts, load_cached_repository_search_result,
        store_cached_repository_search_artifacts, store_cached_repository_search_result,
    };
    use crate::search::{FuzzySearchOptions, SearchDocumentIndex};

    fn ok_or_panic<T, E>(result: Result<T, E>, context: &str) -> T
    where
        E: std::fmt::Display,
    {
        result.unwrap_or_else(|error| panic!("{context}: {error}"))
    }

    fn some_or_panic<T>(value: Option<T>, context: &str) -> T {
        value.unwrap_or_else(|| panic!("{context}"))
    }

    fn sample_analysis_key(repo_id: &str) -> RepositoryAnalysisCacheKey {
        RepositoryAnalysisCacheKey {
            repo_id: repo_id.to_string(),
            checkout_root: format!("/virtual/{repo_id}"),
            checkout_revision: Some("rev-1".to_string()),
            mirror_revision: Some("mirror-1".to_string()),
            tracking_revision: Some("tracking-1".to_string()),
            plugin_ids: vec!["plugin-a".to_string()],
        }
    }

    fn empty_artifacts() -> RepositorySearchArtifacts {
        RepositorySearchArtifacts {
            module_index: SearchDocumentIndex::new(),
            symbol_index: SearchDocumentIndex::new(),
            example_index: SearchDocumentIndex::new(),
            projected_page_index: SearchDocumentIndex::new(),
            modules_by_id: BTreeMap::default(),
            symbols_by_id: BTreeMap::default(),
            examples_by_id: BTreeMap::default(),
            example_metadata: BTreeMap::default(),
            projected_pages_by_id: HashMap::default(),
            projected_pages: Vec::new(),
        }
    }

    #[test]
    fn repository_search_artifacts_cache_roundtrip_uses_analysis_identity() {
        let key = sample_analysis_key("artifact-cache-roundtrip");
        let stored = ok_or_panic(
            store_cached_repository_search_artifacts(key.clone(), empty_artifacts()),
            "artifact cache store should succeed",
        );
        let loaded = some_or_panic(
            ok_or_panic(
                load_cached_repository_search_artifacts(&key),
                "artifact cache load should succeed",
            ),
            "stored artifacts should be present",
        );

        assert!(std::sync::Arc::ptr_eq(&stored, &loaded));
    }

    #[test]
    fn repository_search_query_cache_isolated_by_endpoint_and_filter() {
        let analysis_key = sample_analysis_key("query-cache-isolation");
        let options = FuzzySearchOptions::document_search();
        let module_key = RepositorySearchQueryCacheKey::new(
            &analysis_key,
            "repo.module-search",
            "solve",
            None,
            options,
            10,
        );
        let projected_key = RepositorySearchQueryCacheKey::new(
            &analysis_key,
            "repo.projected-page-search",
            "solve",
            Some("reference".to_string()),
            options,
            10,
        );

        ok_or_panic(
            store_cached_repository_search_result(module_key.clone(), &vec!["module"]),
            "query cache store should succeed",
        );
        ok_or_panic(
            store_cached_repository_search_result(projected_key.clone(), &vec!["projected"]),
            "query cache store should succeed",
        );

        let module_value: Vec<String> = some_or_panic(
            ok_or_panic(
                load_cached_repository_search_result(&module_key),
                "query cache load should succeed",
            ),
            "module cached value should exist",
        );
        let projected_value: Vec<String> = some_or_panic(
            ok_or_panic(
                load_cached_repository_search_result(&projected_key),
                "query cache load should succeed",
            ),
            "projected cached value should exist",
        );

        assert_eq!(module_value, vec!["module".to_string()]);
        assert_eq!(projected_value, vec!["projected".to_string()]);
    }
}
