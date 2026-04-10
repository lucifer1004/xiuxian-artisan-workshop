use crate::gateway::studio::types::SearchHit;
use crate::search::SearchPlaneService;

use super::buffered::search_repo_intent_hits_buffered;
use super::buffered::{RepoSearchResultLimits, search_repo_code_hits_buffered};
use super::dispatch::{RepoSearchDispatch, collect_repo_search_targets, repo_search_parallelism};

use std::time::Duration;

#[derive(Debug, Default)]
pub(crate) struct RepoIntentSearchOutcome {
    pub(crate) hits: Vec<SearchHit>,
    pub(crate) pending_repos: Vec<String>,
    pub(crate) skipped_repos: Vec<String>,
    #[cfg(test)]
    pub(crate) repo_content_available: bool,
}

#[derive(Debug, Default)]
pub(crate) struct RepoCodeSearchOutcome {
    pub(crate) hits: Vec<SearchHit>,
    pub(crate) pending_repos: Vec<String>,
    pub(crate) skipped_repos: Vec<String>,
    pub(crate) partial_timeout: bool,
}

pub(crate) async fn search_repo_intent_outcome(
    search_plane: &SearchPlaneService,
    repo_ids: Vec<String>,
    raw_query: &str,
    limit: usize,
) -> Result<RepoIntentSearchOutcome, String> {
    let dispatch = prepare_repo_search_dispatch(search_plane, repo_ids).await;
    #[cfg(test)]
    let repo_content_available = dispatch
        .searchable
        .iter()
        .any(|target| target.publication_state.content_published);
    let hits = search_repo_intent_hits_buffered(
        search_plane.clone(),
        dispatch.searchable,
        raw_query,
        limit,
    )
    .await?;

    Ok(RepoIntentSearchOutcome {
        hits,
        pending_repos: dispatch.pending,
        skipped_repos: dispatch.skipped,
        #[cfg(test)]
        repo_content_available,
    })
}

pub(crate) async fn search_repo_code_outcome(
    search_plane: &SearchPlaneService,
    repo_ids: Vec<String>,
    raw_query: &str,
    per_repo_limits: RepoSearchResultLimits,
    repo_wide_budget: Option<Duration>,
) -> Result<RepoCodeSearchOutcome, String> {
    let dispatch = prepare_repo_search_dispatch(search_plane, repo_ids).await;
    let buffered = search_repo_code_hits_buffered(
        search_plane.clone(),
        dispatch.searchable,
        raw_query,
        per_repo_limits,
        repo_wide_budget,
    )
    .await?;

    Ok(RepoCodeSearchOutcome {
        hits: buffered.hits,
        pending_repos: dispatch.pending,
        skipped_repos: dispatch.skipped,
        partial_timeout: buffered.partial_timeout,
    })
}

async fn prepare_repo_search_dispatch(
    search_plane: &SearchPlaneService,
    repo_ids: Vec<String>,
) -> RepoSearchDispatch {
    let publication_states = search_plane
        .repo_search_publication_states(repo_ids.as_slice())
        .await;
    let dispatch = collect_repo_search_targets(repo_ids, &publication_states);
    search_plane.record_repo_search_dispatch(
        dispatch.pending.len() + dispatch.skipped.len() + dispatch.searchable.len(),
        dispatch.searchable.len(),
        repo_search_parallelism(search_plane, dispatch.searchable.len()),
    );
    dispatch
}
