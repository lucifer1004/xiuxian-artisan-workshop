use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;
use crate::repo_intelligence::query::{
    ProjectedPageFamilySearchHit, RepoProjectedPageFamilySearchQuery,
    RepoProjectedPageFamilySearchResult,
};

use super::family_context::build_projected_page_family_clusters;
use super::search::scored_projected_page_matches;

/// Build deterministic page-family cluster search results from Repo Intelligence output.
#[must_use]
pub fn build_projected_page_family_search(
    query: &RepoProjectedPageFamilySearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageFamilySearchResult {
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

    RepoProjectedPageFamilySearchResult {
        repo_id: query.repo_id.clone(),
        hits: matches
            .into_iter()
            .take(limit)
            .map(|(_, center_page)| ProjectedPageFamilySearchHit {
                families: build_projected_page_family_clusters(
                    &center_page,
                    analysis,
                    query.per_kind_limit,
                ),
                center_page,
            })
            .collect(),
    }
}
