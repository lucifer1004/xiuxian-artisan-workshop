use crate::repo_intelligence::errors::RepoIntelligenceError;
use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;
use crate::repo_intelligence::query::{
    ProjectedPageIndexNodeContext, ProjectedPageIndexNodeHit, RepoProjectedPageIndexTreeQuery,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalContextResult,
    RepoProjectedRetrievalHitQuery,
};

use super::contracts::{ProjectedPageIndexNode, ProjectedPageIndexTree, ProjectedPageRecord};
use super::related_pages::scored_related_projected_pages;
use super::retrieval_lookup::build_projected_retrieval_hit;
use super::tree_lookup::build_projected_page_index_tree;

/// Build deterministic local retrieval context around one stable Stage-2 hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page.
pub fn build_projected_retrieval_context(
    query: &RepoProjectedRetrievalContextQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    let center = build_projected_retrieval_hit(
        &RepoProjectedRetrievalHitQuery {
            repo_id: query.repo_id.clone(),
            page_id: query.page_id.clone(),
            node_id: query.node_id.clone(),
        },
        analysis,
    )?
    .hit;
    let related_pages = related_projected_pages(&center.page, analysis, query.related_limit);
    let node_context = if let Some(node_id) = &query.node_id {
        let tree = build_projected_page_index_tree(
            &RepoProjectedPageIndexTreeQuery {
                repo_id: query.repo_id.clone(),
                page_id: query.page_id.clone(),
            },
            analysis,
        )?
        .tree;
        Some(build_node_context(
            &center.page,
            &tree,
            query.repo_id.as_str(),
            node_id.as_str(),
        )?)
    } else {
        None
    };

    Ok(RepoProjectedRetrievalContextResult {
        repo_id: query.repo_id.clone(),
        center,
        related_pages,
        node_context,
    })
}

fn related_projected_pages(
    center: &ProjectedPageRecord,
    analysis: &RepositoryAnalysisOutput,
    limit: usize,
) -> Vec<ProjectedPageRecord> {
    if limit == 0 {
        return Vec::new();
    }

    scored_related_projected_pages(center, analysis)
        .into_iter()
        .take(limit)
        .map(|(_, page)| page)
        .collect()
}

fn build_node_context(
    page: &ProjectedPageRecord,
    tree: &ProjectedPageIndexTree,
    repo_id: &str,
    node_id: &str,
) -> Result<ProjectedPageIndexNodeContext, RepoIntelligenceError> {
    let raw = find_node_context(tree.roots.as_slice(), node_id, &[]).ok_or_else(|| {
        RepoIntelligenceError::UnknownProjectedPageIndexNode {
            repo_id: repo_id.to_string(),
            page_id: page.page_id.clone(),
            node_id: node_id.to_string(),
        }
    })?;

    Ok(ProjectedPageIndexNodeContext {
        ancestors: raw
            .ancestors
            .into_iter()
            .map(|node| node_to_hit(repo_id, page, tree, node))
            .collect(),
        previous_sibling: raw
            .previous_sibling
            .map(|node| node_to_hit(repo_id, page, tree, node)),
        next_sibling: raw
            .next_sibling
            .map(|node| node_to_hit(repo_id, page, tree, node)),
        children: raw
            .children
            .into_iter()
            .map(|node| node_to_hit(repo_id, page, tree, node))
            .collect(),
    })
}

struct RawNodeContext<'a> {
    ancestors: Vec<&'a ProjectedPageIndexNode>,
    previous_sibling: Option<&'a ProjectedPageIndexNode>,
    next_sibling: Option<&'a ProjectedPageIndexNode>,
    children: Vec<&'a ProjectedPageIndexNode>,
}

fn find_node_context<'a>(
    nodes: &'a [ProjectedPageIndexNode],
    node_id: &str,
    ancestors: &[&'a ProjectedPageIndexNode],
) -> Option<RawNodeContext<'a>> {
    for (index, node) in nodes.iter().enumerate() {
        if node.node_id == node_id {
            return Some(RawNodeContext {
                ancestors: ancestors.to_vec(),
                previous_sibling: index.checked_sub(1).and_then(|left| nodes.get(left)),
                next_sibling: nodes.get(index + 1),
                children: node.children.iter().collect(),
            });
        }
        let mut child_ancestors = ancestors.to_vec();
        child_ancestors.push(node);
        if let Some(context) =
            find_node_context(node.children.as_slice(), node_id, &child_ancestors)
        {
            return Some(context);
        }
    }
    None
}

fn node_to_hit(
    repo_id: &str,
    page: &ProjectedPageRecord,
    tree: &ProjectedPageIndexTree,
    node: &ProjectedPageIndexNode,
) -> ProjectedPageIndexNodeHit {
    ProjectedPageIndexNodeHit {
        repo_id: repo_id.to_string(),
        page_id: page.page_id.clone(),
        page_title: page.title.clone(),
        page_kind: page.kind,
        path: tree.path.clone(),
        doc_id: tree.doc_id.clone(),
        node_id: node.node_id.clone(),
        node_title: node.title.clone(),
        structural_path: node.structural_path.clone(),
        line_range: node.line_range,
        text: node.text.clone(),
    }
}
