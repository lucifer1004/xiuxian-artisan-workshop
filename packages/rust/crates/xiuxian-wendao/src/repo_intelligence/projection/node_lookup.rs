use crate::repo_intelligence::{
    ProjectedPageIndexNode, RepoIntelligenceError, RepoProjectedPageIndexNodeQuery,
    RepoProjectedPageIndexNodeResult, RepoProjectedPageIndexTreeQuery, RepoProjectedPageQuery,
    RepositoryAnalysisOutput,
};

use super::{build_projected_page, build_projected_page_index_tree};

/// Resolve one deterministic projected page-index node by stable page and node identifiers.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPageIndexNode`] when the requested projected
/// page-index node does not exist for the analyzed repository.
pub fn build_projected_page_index_node(
    query: &RepoProjectedPageIndexNodeQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    let page = build_projected_page(
        &RepoProjectedPageQuery {
            repo_id: query.repo_id.clone(),
            page_id: query.page_id.clone(),
        },
        analysis,
    )?
    .page;
    let tree = build_projected_page_index_tree(
        &RepoProjectedPageIndexTreeQuery {
            repo_id: query.repo_id.clone(),
            page_id: query.page_id.clone(),
        },
        analysis,
    )?
    .tree;

    let hit = find_node(tree.roots.as_slice(), query.node_id.as_str())
        .map(|node| crate::repo_intelligence::ProjectedPageIndexNodeHit {
            repo_id: tree.repo_id.clone(),
            page_id: tree.page_id.clone(),
            page_title: tree.title.clone(),
            page_kind: page.kind,
            path: tree.path.clone(),
            doc_id: tree.doc_id.clone(),
            node_id: node.node_id.clone(),
            node_title: node.title.clone(),
            structural_path: node.structural_path.clone(),
            line_range: node.line_range,
            text: node.text.clone(),
        })
        .ok_or_else(|| RepoIntelligenceError::UnknownProjectedPageIndexNode {
            repo_id: query.repo_id.clone(),
            page_id: query.page_id.clone(),
            node_id: query.node_id.clone(),
        })?;

    Ok(RepoProjectedPageIndexNodeResult {
        repo_id: query.repo_id.clone(),
        hit,
    })
}

fn find_node<'a>(
    nodes: &'a [ProjectedPageIndexNode],
    node_id: &str,
) -> Option<&'a ProjectedPageIndexNode> {
    for node in nodes {
        if node.node_id == node_id {
            return Some(node);
        }
        if let Some(found) = find_node(node.children.as_slice(), node_id) {
            return Some(found);
        }
    }
    None
}
