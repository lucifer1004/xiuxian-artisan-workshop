use crate::repo_intelligence::{
    RepoIntelligenceError, RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeResult,
    RepositoryAnalysisOutput,
};

use super::markdown::build_projected_page_index_trees;

/// Resolve one deterministic projected page-index tree by stable page identifier.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page does
/// not exist for the analyzed repository, or another [`RepoIntelligenceError`] when projected
/// page-index tree construction fails.
pub fn build_projected_page_index_tree(
    query: &RepoProjectedPageIndexTreeQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    let tree = build_projected_page_index_trees(analysis)?
        .into_iter()
        .find(|tree| tree.page_id == query.page_id)
        .ok_or_else(|| RepoIntelligenceError::UnknownProjectedPage {
            repo_id: query.repo_id.clone(),
            page_id: query.page_id.clone(),
        })?;

    Ok(RepoProjectedPageIndexTreeResult {
        repo_id: query.repo_id.clone(),
        tree,
    })
}
