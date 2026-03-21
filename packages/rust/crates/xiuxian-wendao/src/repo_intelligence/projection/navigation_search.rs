use crate::repo_intelligence::errors::RepoIntelligenceError;
use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;
use crate::repo_intelligence::query::{
    ProjectedPageNavigationSearchHit, RepoProjectedPageNavigationQuery,
    RepoProjectedPageNavigationSearchQuery, RepoProjectedPageNavigationSearchResult,
};

use super::navigation_bundle::build_projected_page_navigation;
use super::search::scored_projected_page_matches;

/// Build deterministic projected page-navigation search results from Repo Intelligence output.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when a matched projected page cannot be expanded into a
/// deterministic navigation bundle.
pub fn build_projected_page_navigation_search(
    query: &RepoProjectedPageNavigationSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
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

    let hits = matches
        .into_iter()
        .map(|(search_score, center_page)| {
            match build_projected_page_navigation(
                &RepoProjectedPageNavigationQuery {
                    repo_id: query.repo_id.clone(),
                    page_id: center_page.page_id,
                    node_id: None,
                    family_kind: query.family_kind,
                    related_limit: query.related_limit,
                    family_limit: query.family_limit,
                },
                analysis,
            ) {
                Ok(navigation) => Ok(Some(ProjectedPageNavigationSearchHit {
                    search_score,
                    navigation,
                })),
                Err(RepoIntelligenceError::UnknownProjectedPageFamilyCluster { .. })
                    if query.family_kind.is_some() =>
                {
                    Ok(None)
                }
                Err(error) => Err(error),
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .take(limit)
        .collect();

    Ok(RepoProjectedPageNavigationSearchResult {
        repo_id: query.repo_id.clone(),
        hits,
    })
}
