use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::search::handlers::knowledge::intent::IntentSearchTransportMetadata;
use crate::gateway::studio::types::SearchHit;
use crate::search::repo_search::search_repo_intent_outcome;

#[derive(Debug, Default)]
pub(super) struct RepoIntentMerge {
    pub(super) hits: Vec<SearchHit>,
    pub(super) transport: IntentSearchTransportMetadata,
    pub(super) pending_repos: Vec<String>,
    pub(super) skipped_repos: Vec<String>,
}

pub(super) async fn build_repo_intent_merge(
    studio: &StudioState,
    raw_query: &str,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<RepoIntentMerge, StudioApiError> {
    let repo_ids = if let Some(repo_id) = repo_hint {
        vec![
            configured_repository(studio, repo_id)
                .map_err(map_repo_intelligence_error)?
                .id,
        ]
    } else {
        configured_repositories(studio)
            .into_iter()
            .map(|repository| repository.id)
            .collect::<Vec<_>>()
    };

    let outcome = search_repo_intent_outcome(&studio.search_plane, repo_ids, raw_query, limit)
        .await
        .map_err(|error| {
            StudioApiError::internal(
                "REPO_INTENT_SEARCH_FAILED",
                "Failed to execute shared repo intent orchestration",
                Some(error),
            )
        })?;
    Ok(RepoIntentMerge {
        transport: IntentSearchTransportMetadata {
            #[cfg(test)]
            repo_content_transport: outcome.repo_content_available.then_some("flight_contract"),
        },
        hits: outcome.hits,
        pending_repos: outcome.pending_repos,
        skipped_repos: outcome.skipped_repos,
    })
}
