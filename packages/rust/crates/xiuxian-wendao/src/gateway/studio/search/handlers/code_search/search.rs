use std::collections::VecDeque;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::{Instant, timeout_at};

use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::types::SearchHit;
use crate::gateway::studio::types::SearchResponse;
use crate::search_plane::{
    RepoSearchQueryCacheKeyInput, SearchCorpusKind, SearchPlaneCacheTtl, SearchPlaneService,
};

use super::query::{
    collect_repo_search_targets, parse_code_search_query, parse_repo_code_search_query,
    repo_search_parallelism, repo_search_result_limits, repo_wide_code_search_timeout,
};
use super::types::RepoSearchTarget;

#[derive(Debug, Default)]
struct BufferedRepoSearchResult {
    hits: Vec<SearchHit>,
    partial_timeout: bool,
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn build_code_search_response(
    studio: &StudioState,
    raw_query: String,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<SearchResponse, StudioApiError> {
    build_code_search_response_with_budget(
        studio,
        raw_query,
        repo_hint,
        limit,
        repo_wide_code_search_timeout(repo_hint),
    )
    .await
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn build_code_search_response_with_budget(
    studio: &StudioState,
    raw_query: String,
    repo_hint: Option<&str>,
    limit: usize,
    repo_wide_budget: Option<Duration>,
) -> Result<SearchResponse, StudioApiError> {
    let parsed = parse_code_search_query(raw_query.as_str(), repo_hint);
    let repo_ids = if let Some(repo_id) = parsed.repo.as_deref() {
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

    if repo_ids.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for code search",
        ));
    }
    let cache_key = studio
        .search_plane
        .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
            scope: "code_search",
            corpora: &[],
            repo_corpora: &[
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ],
            repo_ids: repo_ids.as_slice(),
            query: raw_query.as_str(),
            limit,
            intent: Some("code_search"),
            repo_hint: parsed.repo.as_deref(),
        })
        .await;
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let mut hits = Vec::new();
    let publication_states = studio
        .search_plane
        .repo_search_publication_states(repo_ids.as_slice())
        .await;
    let dispatch = collect_repo_search_targets(repo_ids, &publication_states);
    studio.search_plane.record_repo_search_dispatch(
        dispatch.pending_repos.len()
            + dispatch.skipped_repos.len()
            + dispatch.searchable_repos.len(),
        dispatch.searchable_repos.len(),
        repo_search_parallelism(&studio.search_plane, dispatch.searchable_repos.len()),
    );
    let pending_repos = dispatch.pending_repos;
    let skipped_repos = dispatch.skipped_repos;
    let buffered = search_repo_code_hits_buffered(
        studio.search_plane.clone(),
        dispatch.searchable_repos,
        raw_query.as_str(),
        repo_search_result_limits(parsed.repo.as_deref(), limit),
        repo_wide_budget,
    )
    .await?;
    let partial_timeout = buffered.partial_timeout;
    hits.extend(buffered.hits);

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.stem.cmp(&right.stem))
    });
    hits.truncate(limit);

    let hit_count = hits.len();
    let indexing_state = if partial_timeout {
        "partial".to_string()
    } else if pending_repos.is_empty() {
        "ready".to_string()
    } else if hit_count == 0 {
        "indexing".to_string()
    } else {
        "partial".to_string()
    };

    let response = SearchResponse {
        query: raw_query,
        hit_count,
        hits,
        graph_confidence_score: None,
        selected_mode: Some("code_search".to_string()),
        intent: Some("code_search".to_string()),
        intent_confidence: Some(1.0),
        search_mode: Some("code_search".to_string()),
        partial: partial_timeout || !pending_repos.is_empty() || !skipped_repos.is_empty(),
        indexing_state: Some(indexing_state),
        pending_repos,
        skipped_repos,
    };
    if let Some(cache_key) = cache_key.as_ref() {
        studio
            .search_plane
            .cache_set_json(cache_key, SearchPlaneCacheTtl::HotQuery, &response)
            .await;
    }
    Ok(response)
}

pub(crate) async fn search_repo_entity_hits(
    search_plane: &SearchPlaneService,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    match search_plane
        .search_repo_entities(
            repo_id,
            search_term,
            &parsed.language_filters,
            &parsed.kind_filters,
            limit,
        )
        .await
    {
        Ok(hits) => Ok(hits),
        Err(error) => Err(StudioApiError::internal(
            "REPO_ENTITY_SEARCH_FAILED",
            "Failed to query repo entity search plane",
            Some(error.to_string()),
        )),
    }
}

pub(crate) async fn search_repo_content_hits(
    search_plane: &SearchPlaneService,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    if !parsed.kind_filters.is_empty() && !parsed.kind_filters.contains("file") {
        return Ok(Vec::new());
    }
    match search_plane
        .search_repo_content_chunks(repo_id, search_term, &parsed.language_filters, limit)
        .await
    {
        Ok(hits) => Ok(hits),
        Err(error) => Err(StudioApiError::internal(
            "REPO_CONTENT_SEARCH_FAILED",
            "Failed to query repo content search plane",
            Some(error.to_string()),
        )),
    }
}

#[cfg(test)]
pub(crate) async fn build_repo_entity_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_entity_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

#[cfg(test)]
pub(crate) async fn build_repo_content_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_content_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

async fn search_repo_code_hits_buffered(
    search_plane: SearchPlaneService,
    targets: Vec<RepoSearchTarget>,
    raw_query: &str,
    per_repo_limits: super::query::RepoSearchResultLimits,
    repo_wide_budget: Option<Duration>,
) -> Result<BufferedRepoSearchResult, StudioApiError> {
    if targets.is_empty() {
        return Ok(BufferedRepoSearchResult::default());
    }

    let mut queued = VecDeque::from(targets);
    let mut join_set = JoinSet::new();
    let raw_query = raw_query.to_string();
    let parallelism = repo_search_parallelism(&search_plane, queued.len());
    let deadline = repo_wide_budget.map(|budget| Instant::now() + budget);
    for _ in 0..parallelism {
        if let Some(target) = queued.pop_front() {
            spawn_repo_code_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                per_repo_limits,
            );
        }
    }

    let mut hits = Vec::new();
    while !join_set.is_empty() {
        let next_result = if let Some(deadline) = deadline {
            match timeout_at(deadline, join_set.join_next()).await {
                Ok(result) => result,
                Err(_) => {
                    join_set.abort_all();
                    while join_set.join_next().await.is_some() {}
                    return Ok(BufferedRepoSearchResult {
                        hits,
                        partial_timeout: true,
                    });
                }
            }
        } else {
            join_set.join_next().await
        };
        let Some(result) = next_result else {
            break;
        };
        let repository_hits = result.map_err(|error| {
            StudioApiError::internal(
                "REPO_CODE_SEARCH_TASK_FAILED",
                "Repo code-search task failed",
                Some(error.to_string()),
            )
        })??;
        hits.extend(repository_hits);
        if let Some(target) = queued.pop_front() {
            spawn_repo_code_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                per_repo_limits,
            );
        }
    }
    Ok(BufferedRepoSearchResult {
        hits,
        partial_timeout: false,
    })
}

fn spawn_repo_code_search_task(
    join_set: &mut JoinSet<Result<Vec<SearchHit>, StudioApiError>>,
    search_plane: SearchPlaneService,
    target: RepoSearchTarget,
    raw_query: String,
    per_repo_limits: super::query::RepoSearchResultLimits,
) {
    join_set.spawn(async move {
        let mut repository_hits = if target.publication_state.entity_published {
            search_repo_entity_hits(
                &search_plane,
                target.repo_id.as_str(),
                raw_query.as_str(),
                per_repo_limits.entity_limit,
            )
            .await?
        } else {
            Vec::new()
        };

        if repository_hits.is_empty() && target.publication_state.content_published {
            repository_hits.extend(
                search_repo_content_hits(
                    &search_plane,
                    target.repo_id.as_str(),
                    raw_query.as_str(),
                    per_repo_limits.content_limit,
                )
                .await?,
            );
        }

        Ok(repository_hits)
    });
}
