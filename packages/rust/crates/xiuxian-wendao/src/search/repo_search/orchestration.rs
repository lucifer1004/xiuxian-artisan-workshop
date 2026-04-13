use crate::analyzers::RegisteredRepository;
use crate::gateway::studio::types::SearchHit;
use crate::parsers::search::repo_code_query::ParsedRepoCodeSearchQuery;
use crate::search::SearchPlaneService;

use super::ast::{
    ast_pattern_requests_generic_analysis, has_generic_ast_language_filters,
    repository_supports_generic_ast_analysis, search_repo_ast_analysis_hits,
    search_repo_ast_pattern_hits,
};
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepoCodeSearchExecutionError {
    #[error("ast-grep code search requires one explicit repository scope")]
    MissingRepositoryScopeForAstGrep,
    #[error("{0}")]
    Search(String),
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

pub(crate) async fn search_repo_code_outcome_for_query(
    search_plane: &SearchPlaneService,
    selected_repository: Option<&RegisteredRepository>,
    repo_ids: Vec<String>,
    raw_query: &str,
    parsed_query: &ParsedRepoCodeSearchQuery,
    per_repo_limits: RepoSearchResultLimits,
    repo_wide_budget: Option<Duration>,
) -> Result<RepoCodeSearchOutcome, RepoCodeSearchExecutionError> {
    if let Some(ast_pattern) = parsed_query.ast_pattern.as_deref() {
        let repository = selected_repository
            .ok_or(RepoCodeSearchExecutionError::MissingRepositoryScopeForAstGrep)?;
        let language_filters = parsed_query
            .language_filters
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if ast_pattern_requests_generic_analysis(ast_pattern) {
            let hits = search_repo_ast_analysis_hits(
                search_plane,
                repository,
                parsed_query.search_term(),
                language_filters.as_slice(),
                per_repo_limits.entity_limit,
            )
            .await
            .map_err(RepoCodeSearchExecutionError::Search)?;
            return Ok(RepoCodeSearchOutcome {
                hits,
                pending_repos: Vec::new(),
                skipped_repos: Vec::new(),
                partial_timeout: false,
            });
        }
        let hits = search_repo_ast_pattern_hits(
            search_plane,
            repository,
            ast_pattern,
            language_filters.as_slice(),
            per_repo_limits.entity_limit,
        )
        .await
        .map_err(RepoCodeSearchExecutionError::Search)?;
        return Ok(RepoCodeSearchOutcome {
            hits,
            pending_repos: Vec::new(),
            skipped_repos: Vec::new(),
            partial_timeout: false,
        });
    }

    if let Some(repository) = selected_repository
        && repository_supports_generic_ast_analysis(repository)
        && has_generic_ast_language_filters(repository, &parsed_query.language_filters)
        && parsed_query.kind_filters.is_empty()
    {
        let language_filters = parsed_query
            .language_filters
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let hits = search_repo_ast_analysis_hits(
            search_plane,
            repository,
            parsed_query.search_term(),
            language_filters.as_slice(),
            per_repo_limits.entity_limit,
        )
        .await
        .map_err(RepoCodeSearchExecutionError::Search)?;
        return Ok(RepoCodeSearchOutcome {
            hits,
            pending_repos: Vec::new(),
            skipped_repos: Vec::new(),
            partial_timeout: false,
        });
    }

    search_repo_code_outcome(
        search_plane,
        repo_ids,
        raw_query,
        per_repo_limits,
        repo_wide_budget,
    )
    .await
    .map_err(RepoCodeSearchExecutionError::Search)
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
