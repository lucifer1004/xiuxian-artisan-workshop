use std::path::Path;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::projection::{
    build_projected_page_index_node, build_projected_page_index_tree,
    build_projected_page_index_tree_search, build_projected_page_index_trees,
};
use crate::analyzers::query::{
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexNodeResult,
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeResult,
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageIndexTreeSearchResult,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageIndexTreesResult,
};
use crate::analyzers::registry::PluginRegistry;

use super::registry::{with_bootstrapped_repository_analysis, with_repository_analysis};

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
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        build_repo_projected_page_index_tree(query, analysis)
    })
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
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        build_repo_projected_page_index_tree(query, analysis)
    })
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
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        build_repo_projected_page_index_node(query, analysis)
    })
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
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        build_repo_projected_page_index_node(query, analysis)
    })
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
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        Ok(build_repo_projected_page_index_tree_search(query, analysis))
    })
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
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        Ok(build_repo_projected_page_index_tree_search(query, analysis))
    })
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
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        build_repo_projected_page_index_trees(query, analysis)
    })
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
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        build_repo_projected_page_index_trees(query, analysis)
    })
}
