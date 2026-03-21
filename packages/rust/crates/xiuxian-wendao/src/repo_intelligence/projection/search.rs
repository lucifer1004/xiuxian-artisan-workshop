use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;
use crate::repo_intelligence::query::{
    RepoProjectedPageSearchQuery, RepoProjectedPageSearchResult,
};

use super::contracts::{ProjectedPageRecord, ProjectionPageKind};
use super::pages::build_projected_pages;

/// Build deterministic projected-page search results from Repo Intelligence output.
#[must_use]
pub fn build_projected_page_search(
    query: &RepoProjectedPageSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageSearchResult {
    let normalized_query = query.query.trim().to_ascii_lowercase();
    let limit = query.limit.max(1);
    let mut matches =
        scored_projected_page_matches(normalized_query.as_str(), query.kind, analysis);

    matches.sort_by(|(left_score, left_page), (right_score, right_page)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_page.title.cmp(&right_page.title))
            .then_with(|| left_page.page_id.cmp(&right_page.page_id))
    });

    RepoProjectedPageSearchResult {
        repo_id: query.repo_id.clone(),
        pages: matches
            .into_iter()
            .take(limit)
            .map(|(_, page)| page)
            .collect(),
    }
}

#[must_use]
pub(crate) fn scored_projected_page_matches(
    query: &str,
    expected_kind: Option<ProjectionPageKind>,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<(u8, ProjectedPageRecord)> {
    build_projected_pages(analysis)
        .into_iter()
        .filter_map(|page| {
            let score = projected_page_match_score(query, expected_kind, &page)?;
            Some((score, page))
        })
        .collect()
}

pub(crate) fn projected_page_match_score(
    query: &str,
    expected_kind: Option<ProjectionPageKind>,
    page: &ProjectedPageRecord,
) -> Option<u8> {
    if expected_kind.is_some_and(|kind| kind != page.kind) {
        return None;
    }

    if query.is_empty() {
        return Some(0);
    }

    let title = page.title.to_ascii_lowercase();
    if title == query {
        return Some(0);
    }
    if title.starts_with(query) {
        return Some(1);
    }
    if title.contains(query) {
        return Some(2);
    }
    if page
        .paths
        .iter()
        .any(|path| path.to_ascii_lowercase().contains(query))
    {
        return Some(3);
    }
    if page
        .format_hints
        .iter()
        .any(|hint| hint.to_ascii_lowercase().contains(query))
    {
        return Some(4);
    }
    if page
        .sections
        .iter()
        .any(|section| section.title.to_ascii_lowercase().contains(query))
    {
        return Some(5);
    }

    None
}
