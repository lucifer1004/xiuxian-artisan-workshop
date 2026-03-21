use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;
use crate::repo_intelligence::query::{
    ProjectedPageIndexNodeHit, RepoProjectedPageIndexTreeSearchQuery,
    RepoProjectedPageIndexTreeSearchResult,
};

use super::contracts::{ProjectedPageIndexNode, ProjectionPageKind};
use super::markdown::build_projected_page_index_trees;
use super::pages::build_projected_pages;

/// Build deterministic section-level retrieval hits from projected page-index trees.
#[must_use]
pub fn build_projected_page_index_tree_search(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageIndexTreeSearchResult {
    let normalized_query = query.query.trim().to_ascii_lowercase();
    let limit = query.limit.max(1);
    let mut hits =
        scored_projected_page_index_node_hits(normalized_query.as_str(), query.kind, analysis);

    hits.sort_by(|(left_score, left_hit), (right_score, right_hit)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_hit.page_title.cmp(&right_hit.page_title))
            .then_with(|| left_hit.structural_path.cmp(&right_hit.structural_path))
            .then_with(|| left_hit.node_id.cmp(&right_hit.node_id))
    });

    RepoProjectedPageIndexTreeSearchResult {
        repo_id: query.repo_id.clone(),
        hits: hits.into_iter().take(limit).map(|(_, hit)| hit).collect(),
    }
}

#[derive(Clone)]
pub(crate) struct PageTreeContext {
    repo_id: String,
    page_id: String,
    page_title: String,
    page_kind: ProjectionPageKind,
    path: String,
    doc_id: String,
}

#[must_use]
pub(crate) fn scored_projected_page_index_node_hits(
    query: &str,
    expected_kind: Option<ProjectionPageKind>,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<(u8, ProjectedPageIndexNodeHit)> {
    let pages = build_projected_pages(analysis)
        .into_iter()
        .map(|page| (page.page_id.clone(), (page.kind, page.title)))
        .collect::<std::collections::BTreeMap<_, _>>();
    build_projected_page_index_trees(analysis)
        .unwrap_or_default()
        .into_iter()
        .flat_map(|tree| {
            let Some((page_kind, page_title)) = pages.get(&tree.page_id).cloned() else {
                return Vec::new();
            };
            let context = PageTreeContext {
                repo_id: tree.repo_id,
                page_id: tree.page_id,
                page_title,
                page_kind,
                path: tree.path,
                doc_id: tree.doc_id,
            };
            collect_node_hits(query, expected_kind, &context, tree.roots.as_slice())
        })
        .collect()
}

pub(crate) fn collect_node_hits(
    query: &str,
    expected_kind: Option<ProjectionPageKind>,
    context: &PageTreeContext,
    nodes: &[ProjectedPageIndexNode],
) -> Vec<(u8, ProjectedPageIndexNodeHit)> {
    let mut hits = Vec::new();
    for node in nodes {
        if let Some(score) =
            projected_page_index_node_match_score(query, expected_kind, context, node)
        {
            hits.push((
                score,
                ProjectedPageIndexNodeHit {
                    repo_id: context.repo_id.clone(),
                    page_id: context.page_id.clone(),
                    page_title: context.page_title.clone(),
                    page_kind: context.page_kind,
                    path: context.path.clone(),
                    doc_id: context.doc_id.clone(),
                    node_id: node.node_id.clone(),
                    node_title: node.title.clone(),
                    structural_path: node.structural_path.clone(),
                    line_range: node.line_range,
                    text: node.text.clone(),
                },
            ));
        }
        hits.extend(collect_node_hits(
            query,
            expected_kind,
            context,
            node.children.as_slice(),
        ));
    }
    hits
}

pub(crate) fn projected_page_index_node_match_score(
    query: &str,
    expected_kind: Option<ProjectionPageKind>,
    context: &PageTreeContext,
    node: &ProjectedPageIndexNode,
) -> Option<u8> {
    if expected_kind.is_some_and(|kind| kind != context.page_kind) {
        return None;
    }
    if query.is_empty() {
        return Some(0);
    }

    let node_title = node.title.to_ascii_lowercase();
    if node_title == query {
        return Some(0);
    }
    if node_title.starts_with(query) {
        return Some(1);
    }
    if node_title.contains(query) {
        return Some(2);
    }
    if node
        .structural_path
        .iter()
        .any(|segment| segment.to_ascii_lowercase().contains(query))
    {
        return Some(3);
    }
    if context.page_title.to_ascii_lowercase().contains(query) {
        return Some(4);
    }
    if node.text.to_ascii_lowercase().contains(query) {
        return Some(5);
    }
    if context.path.to_ascii_lowercase().contains(query) {
        return Some(6);
    }

    None
}
