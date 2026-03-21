use crate::repo_intelligence::errors::RepoIntelligenceError;
use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;
use crate::repo_intelligence::query::{
    ProjectedRetrievalHit, ProjectedRetrievalHitKind, RepoProjectedPageIndexNodeQuery,
    RepoProjectedPageQuery, RepoProjectedRetrievalHitQuery, RepoProjectedRetrievalHitResult,
};

use super::lookup::build_projected_page;
use super::node_lookup::build_projected_page_index_node;

/// Build one deterministic mixed retrieval hit from stable projected identifiers.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page.
pub fn build_projected_retrieval_hit(
    query: &RepoProjectedRetrievalHitQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    let page = build_projected_page(
        &RepoProjectedPageQuery {
            repo_id: query.repo_id.clone(),
            page_id: query.page_id.clone(),
        },
        analysis,
    )?
    .page;

    let hit = if let Some(node_id) = &query.node_id {
        let node = build_projected_page_index_node(
            &RepoProjectedPageIndexNodeQuery {
                repo_id: query.repo_id.clone(),
                page_id: query.page_id.clone(),
                node_id: node_id.clone(),
            },
            analysis,
        )?
        .hit;
        ProjectedRetrievalHit {
            kind: ProjectedRetrievalHitKind::PageIndexNode,
            page,
            node: Some(node),
        }
    } else {
        ProjectedRetrievalHit {
            kind: ProjectedRetrievalHitKind::Page,
            page,
            node: None,
        }
    };

    Ok(RepoProjectedRetrievalHitResult {
        repo_id: query.repo_id.clone(),
        hit,
    })
}
