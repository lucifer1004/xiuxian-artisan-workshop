use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::cache::{
    load_cached_repository_search_result, store_cached_repository_search_result,
};
use crate::analyzers::service::{build_example_search_with_artifacts, repository_search_artifacts};
use crate::analyzers::{ExampleSearchQuery, RepoIntelligenceError};
use crate::gateway::studio::router::handlers::repo::analysis::search::cache::{
    repository_search_key, with_cached_repo_search_result,
};
use crate::gateway::studio::router::handlers::repo::analysis::search::publication::repo_entity_publication_ready;
use crate::gateway::studio::router::handlers::repo::{
    required_repo_id, required_search_query, shared::with_repo_cached_analysis_bundle,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::search::FuzzySearchOptions;
use crate::search_plane::search_repo_entity_example_results;

/// Example search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, repository lookup or
/// analysis fails, or the background task panics.
pub async fn example_search(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ExampleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let search_plane = state.studio.search_plane.clone();
    let cache_repo_id = repo_id.clone();
    let cache_query = search_query.clone();
    let result = with_cached_repo_search_result(
        &search_plane,
        "repo.example-search",
        cache_repo_id.as_str(),
        cache_query.as_str(),
        limit,
        {
            let state = Arc::clone(&state);
            let repo_id = repo_id.clone();
            let search_query = search_query.clone();
            move || async move {
                if let Some(result) = search_repo_examples_with_search_plane(
                    Arc::clone(&state),
                    repo_id.as_str(),
                    search_query.as_str(),
                    limit,
                )
                .await?
                {
                    return Ok(result);
                }
                with_repo_cached_analysis_bundle(
                    Arc::clone(&state),
                    repo_id.clone(),
                    "REPO_EXAMPLE_SEARCH_PANIC",
                    "Repo example search task failed unexpectedly",
                    move |cached| {
                        let query = ExampleSearchQuery {
                            repo_id,
                            query: search_query,
                            limit,
                        };
                        let cache_key = repository_search_key(
                            &cached.cache_key,
                            "repo.example-search",
                            query.query.as_str(),
                            query.limit,
                            FuzzySearchOptions::document_search(),
                        );
                        if let Some(result) = load_cached_repository_search_result(&cache_key)? {
                            return Ok(result);
                        }

                        let artifacts =
                            repository_search_artifacts(&cached.cache_key, &cached.analysis)?;
                        let result = build_example_search_with_artifacts(
                            &query,
                            &cached.analysis,
                            artifacts.as_ref(),
                        );
                        store_cached_repository_search_result(cache_key, &result)?;
                        Ok::<_, RepoIntelligenceError>(result)
                    },
                )
                .await
            }
        },
    )
    .await?;
    Ok(Json(result))
}

async fn search_repo_examples_with_search_plane(
    state: Arc<crate::gateway::studio::router::GatewayState>,
    repo_id: &str,
    query: &str,
    limit: usize,
) -> Result<Option<crate::analyzers::ExampleSearchResult>, StudioApiError> {
    if !repo_entity_publication_ready(&state, repo_id).await {
        return Ok(None);
    }
    search_repo_entity_example_results(&state.studio.search_plane, repo_id, query, limit)
        .await
        .map(Some)
        .map_err(|error| {
            StudioApiError::internal(
                "REPO_EXAMPLE_SEARCH_FAILED",
                "Repo example search task failed",
                Some(error.to_string()),
            )
        })
}
