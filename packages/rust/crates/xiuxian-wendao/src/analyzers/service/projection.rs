//! Repository projection functions (projected pages, retrieval, navigation, index trees).

use std::path::Path;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::projection::{
    build_projected_page, build_projected_page_family_cluster, build_projected_page_family_context,
    build_projected_page_family_search, build_projected_page_index_node,
    build_projected_page_index_tree, build_projected_page_index_tree_search,
    build_projected_page_index_trees, build_projected_page_navigation,
    build_projected_page_navigation_search, build_projected_page_search, build_projected_pages,
    build_projected_retrieval, build_projected_retrieval_context, build_projected_retrieval_hit,
};
use crate::analyzers::query::{
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyClusterResult,
    RepoProjectedPageFamilyContextQuery, RepoProjectedPageFamilyContextResult,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageFamilySearchResult,
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexNodeResult,
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeResult,
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageIndexTreeSearchResult,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageIndexTreesResult,
    RepoProjectedPageNavigationQuery, RepoProjectedPageNavigationResult,
    RepoProjectedPageNavigationSearchQuery, RepoProjectedPageNavigationSearchResult,
    RepoProjectedPageQuery, RepoProjectedPageResult, RepoProjectedPageSearchQuery,
    RepoProjectedPageSearchResult, RepoProjectedPagesQuery, RepoProjectedPagesResult,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalContextResult,
    RepoProjectedRetrievalHitQuery, RepoProjectedRetrievalHitResult, RepoProjectedRetrievalQuery,
    RepoProjectedRetrievalResult,
};
use crate::analyzers::registry::PluginRegistry;

use super::{analyze_repository_from_config_with_registry, bootstrap_builtin_registry};

/// Build deterministic projected pages from normalized analysis records.
#[must_use]
pub fn build_repo_projected_pages(
    query: &RepoProjectedPagesQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPagesResult {
    RepoProjectedPagesResult {
        repo_id: query.repo_id.clone(),
        pages: build_projected_pages(analysis),
    }
}

/// Load configuration, analyze one repository, and return deterministic projected pages.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_pages_from_config_with_registry(
    query: &RepoProjectedPagesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPagesResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_pages(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic projected pages.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_pages_from_config(
    query: &RepoProjectedPagesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPagesResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_pages_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic projected page from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output.
pub fn build_repo_projected_page(
    query: &RepoProjectedPageQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageResult, RepoIntelligenceError> {
    build_projected_page(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_from_config_with_registry(
    query: &RepoProjectedPageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_from_config(
    query: &RepoProjectedPageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic page-family context around one stable projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output.
pub fn build_repo_projected_page_family_context(
    query: &RepoProjectedPageFamilyContextQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageFamilyContextResult, RepoIntelligenceError> {
    build_projected_page_family_context(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic page-family context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_family_context_from_config_with_registry(
    query: &RepoProjectedPageFamilyContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageFamilyContextResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_family_context(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic page-family context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_family_context_from_config(
    query: &RepoProjectedPageFamilyContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageFamilyContextResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_family_context_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic page-family cluster around one stable projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or
/// [`RepoIntelligenceError::UnknownProjectedPageFamilyCluster`] when the requested family is not
/// present for the projected page.
pub fn build_repo_projected_page_family_cluster(
    query: &RepoProjectedPageFamilyClusterQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageFamilyClusterResult, RepoIntelligenceError> {
    build_projected_page_family_cluster(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-family cluster.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page or family cluster is not present for the repository.
pub fn repo_projected_page_family_cluster_from_config_with_registry(
    query: &RepoProjectedPageFamilyClusterQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageFamilyClusterResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_family_cluster(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-family cluster.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page or family cluster is not present for the repository.
pub fn repo_projected_page_family_cluster_from_config(
    query: &RepoProjectedPageFamilyClusterQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageFamilyClusterResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_family_cluster_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic page-centric Stage-2 navigation bundle.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page, or
/// [`RepoIntelligenceError::UnknownProjectedPageFamilyCluster`] when the requested family is not
/// present for the projected page.
pub fn build_repo_projected_page_navigation(
    query: &RepoProjectedPageNavigationQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageNavigationResult, RepoIntelligenceError> {
    build_projected_page_navigation(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-centric
/// Stage-2 navigation bundle.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page, node, or family cluster is not present for the repository.
pub fn repo_projected_page_navigation_from_config_with_registry(
    query: &RepoProjectedPageNavigationQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageNavigationResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_navigation(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-centric
/// Stage-2 navigation bundle.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page, node, or family cluster is not present for the repository.
pub fn repo_projected_page_navigation_from_config(
    query: &RepoProjectedPageNavigationQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageNavigationResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_navigation_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected page-navigation search results from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when a matched projected page cannot be expanded into a
/// deterministic navigation bundle.
pub fn build_repo_projected_page_navigation_search(
    query: &RepoProjectedPageNavigationSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
    build_projected_page_navigation_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-navigation
/// search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or a matched projected page
/// cannot be expanded into a deterministic navigation bundle.
pub fn repo_projected_page_navigation_search_from_config_with_registry(
    query: &RepoProjectedPageNavigationSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_navigation_search(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-navigation
/// search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or a matched projected page
/// cannot be expanded into a deterministic navigation bundle.
pub fn repo_projected_page_navigation_search_from_config(
    query: &RepoProjectedPageNavigationSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_navigation_search_from_config_with_registry(
        query,
        config_path,
        cwd,
        &registry,
    )
}

/// Build deterministic projected-page search results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_page_search(
    query: &RepoProjectedPageSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageSearchResult {
    build_projected_page_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected-page search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_search_from_config_with_registry(
    query: &RepoProjectedPageSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_page_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic projected-page search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_search_from_config(
    query: &RepoProjectedPageSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic projected page-index node from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPageIndexNode`] when the requested projected
/// page-index node is not present in the analysis output.
pub fn build_repo_projected_page_index_node(
    query: &RepoProjectedPageIndexNodeQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    build_projected_page_index_node(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index node.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page-index node identifier is not present for the repository.
pub fn repo_projected_page_index_node_from_config_with_registry(
    query: &RepoProjectedPageIndexNodeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_index_node(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index node.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page-index node identifier is not present for the repository.
pub fn repo_projected_page_index_node_from_config(
    query: &RepoProjectedPageIndexNodeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_node_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic mixed retrieval results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_retrieval(
    query: &RepoProjectedRetrievalQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedRetrievalResult {
    build_projected_retrieval(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic mixed retrieval results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_retrieval_from_config_with_registry(
    query: &RepoProjectedRetrievalQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedRetrievalResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_retrieval(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic mixed retrieval results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_retrieval_from_config(
    query: &RepoProjectedRetrievalQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedRetrievalResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_retrieval_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic mixed retrieval hit from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page.
pub fn build_repo_projected_retrieval_hit(
    query: &RepoProjectedRetrievalHitQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    build_projected_retrieval_hit(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic mixed retrieval hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_hit_from_config_with_registry(
    query: &RepoProjectedRetrievalHitQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_retrieval_hit(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic mixed retrieval hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_hit_from_config(
    query: &RepoProjectedRetrievalHitQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_retrieval_hit_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic local retrieval context around one stable Stage-2 hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page.
pub fn build_repo_projected_retrieval_context(
    query: &RepoProjectedRetrievalContextQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    build_projected_retrieval_context(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic local retrieval context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_context_from_config_with_registry(
    query: &RepoProjectedRetrievalContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_retrieval_context(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic local retrieval context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_context_from_config(
    query: &RepoProjectedRetrievalContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_retrieval_context_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic projected page-index tree from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or another [`RepoIntelligenceError`] when projected page
/// markdown cannot be parsed into page-index trees.
pub fn build_repo_projected_page_index_tree(
    query: &RepoProjectedPageIndexTreeQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    build_projected_page_index_tree(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index tree.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails, the requested projected page
/// identifier is not present for the repository, or projected page-index tree construction fails.
pub fn repo_projected_page_index_tree_from_config_with_registry(
    query: &RepoProjectedPageIndexTreeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_index_tree(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index tree.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails, the requested projected page
/// identifier is not present for the repository, or projected page-index tree construction fails.
pub fn repo_projected_page_index_tree_from_config(
    query: &RepoProjectedPageIndexTreeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_tree_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected page-index tree search results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_page_index_tree_search(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageIndexTreeSearchResult {
    build_projected_page_index_tree_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-index tree search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_tree_search_from_config_with_registry(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexTreeSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_page_index_tree_search(
        query, &analysis,
    ))
}

/// Load configuration, analyze one repository, and return deterministic projected page-index tree search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_tree_search_from_config(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexTreeSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_tree_search_from_config_with_registry(
        query,
        config_path,
        cwd,
        &registry,
    )
}

/// Build deterministic page-family cluster search results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_page_family_search(
    query: &RepoProjectedPageFamilySearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageFamilySearchResult {
    build_projected_page_family_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic page-family cluster search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_family_search_from_config_with_registry(
    query: &RepoProjectedPageFamilySearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageFamilySearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_page_family_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic page-family cluster search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_family_search_from_config(
    query: &RepoProjectedPageFamilySearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageFamilySearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_family_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected page-index trees from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when projected page markdown cannot be parsed into
/// page-index trees.
pub fn build_repo_projected_page_index_trees(
    query: &RepoProjectedPageIndexTreesQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexTreesResult, RepoIntelligenceError> {
    Ok(RepoProjectedPageIndexTreesResult {
        repo_id: query.repo_id.clone(),
        trees: build_projected_page_index_trees(analysis)?,
    })
}

/// Load configuration, analyze one repository, and return deterministic projected page-index trees.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_trees_from_config_with_registry(
    query: &RepoProjectedPageIndexTreesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexTreesResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_index_trees(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-index trees.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_trees_from_config(
    query: &RepoProjectedPageIndexTreesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexTreesResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_trees_from_config_with_registry(query, config_path, cwd, &registry)
}
