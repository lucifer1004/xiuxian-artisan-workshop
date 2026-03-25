use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::cache::{
    RepositorySearchQueryCacheKey, load_cached_repository_search_result,
    store_cached_repository_search_result,
};
use crate::analyzers::service::{
    build_example_search_with_artifacts, build_module_search_with_artifacts,
    build_symbol_search_with_artifacts, repository_search_artifacts,
};
use crate::analyzers::{
    DocCoverageQuery, ExampleSearchQuery, ModuleSearchQuery, RepoOverviewQuery, RepoSyncQuery,
    SymbolSearchQuery, build_doc_coverage, build_repo_overview,
    repo_sync_for_registered_repository,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::search::FuzzySearchOptions;
use crate::search_plane::{
    SearchCorpusKind, search_repo_entity_example_results, search_repo_entity_module_results,
    search_repo_entity_symbol_results,
};

use super::parse::{parse_repo_sync_mode, required_repo_id, required_search_query};
use super::query::{RepoApiQuery, RepoDocCoverageApiQuery, RepoSearchApiQuery, RepoSyncApiQuery};
use super::shared::{with_repo_analysis, with_repo_cached_analysis_bundle, with_repository};

/// Repository overview endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup fails, repository
/// analysis fails, or the background task panics.
pub async fn overview(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoOverviewResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_OVERVIEW_PANIC",
        "Repo overview task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_repo_overview(
                &RepoOverviewQuery { repo_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Module search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, repository lookup or
/// analysis fails, or the background task panics.
pub async fn module_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ModuleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    if let Some(result) = search_repo_modules_with_search_plane(
        Arc::clone(&state),
        repo_id.as_str(),
        search_query.as_str(),
        limit,
    )
    .await?
    {
        return Ok(Json(result));
    }
    let result = with_repo_cached_analysis_bundle(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_MODULE_SEARCH_PANIC",
        "Repo module search task failed unexpectedly",
        move |cached| {
            let query = ModuleSearchQuery {
                repo_id,
                query: search_query,
                limit,
            };
            let cache_key = RepositorySearchQueryCacheKey::new(
                &cached.cache_key,
                "repo.module-search",
                query.query.as_str(),
                None,
                FuzzySearchOptions::path_search(),
                query.limit,
            );
            if let Some(result) = load_cached_repository_search_result(&cache_key)? {
                return Ok(result);
            }

            let artifacts = repository_search_artifacts(&cached.cache_key, &cached.analysis)?;
            let result =
                build_module_search_with_artifacts(&query, &cached.analysis, artifacts.as_ref());
            store_cached_repository_search_result(cache_key, &result)?;
            Ok::<_, crate::analyzers::RepoIntelligenceError>(result)
        },
    )
    .await?;
    Ok(Json(result))
}

/// Symbol search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, repository lookup or
/// analysis fails, or the background task panics.
pub async fn symbol_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::SymbolSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    if let Some(result) = search_repo_symbols_with_search_plane(
        Arc::clone(&state),
        repo_id.as_str(),
        search_query.as_str(),
        limit,
    )
    .await?
    {
        return Ok(Json(result));
    }
    let result = with_repo_cached_analysis_bundle(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_SYMBOL_SEARCH_PANIC",
        "Repo symbol search task failed unexpectedly",
        move |cached| {
            let query = SymbolSearchQuery {
                repo_id,
                query: search_query,
                limit,
            };
            let cache_key = RepositorySearchQueryCacheKey::new(
                &cached.cache_key,
                "repo.symbol-search",
                query.query.as_str(),
                None,
                FuzzySearchOptions::symbol_search(),
                query.limit,
            );
            if let Some(result) = load_cached_repository_search_result(&cache_key)? {
                return Ok(result);
            }

            let artifacts = repository_search_artifacts(&cached.cache_key, &cached.analysis)?;
            let result =
                build_symbol_search_with_artifacts(&query, &cached.analysis, artifacts.as_ref());
            store_cached_repository_search_result(cache_key, &result)?;
            Ok::<_, crate::analyzers::RepoIntelligenceError>(result)
        },
    )
    .await?;
    Ok(Json(result))
}

/// Example search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, repository lookup or
/// analysis fails, or the background task panics.
pub async fn example_search(
    Query(query): Query<RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ExampleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    if let Some(result) = search_repo_examples_with_search_plane(
        Arc::clone(&state),
        repo_id.as_str(),
        search_query.as_str(),
        limit,
    )
    .await?
    {
        return Ok(Json(result));
    }
    let result = with_repo_cached_analysis_bundle(
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
            let cache_key = RepositorySearchQueryCacheKey::new(
                &cached.cache_key,
                "repo.example-search",
                query.query.as_str(),
                None,
                FuzzySearchOptions::document_search(),
                query.limit,
            );
            if let Some(result) = load_cached_repository_search_result(&cache_key)? {
                return Ok(result);
            }

            let artifacts = repository_search_artifacts(&cached.cache_key, &cached.analysis)?;
            let result =
                build_example_search_with_artifacts(&query, &cached.analysis, artifacts.as_ref());
            store_cached_repository_search_result(cache_key, &result)?;
            Ok::<_, crate::analyzers::RepoIntelligenceError>(result)
        },
    )
    .await?;
    Ok(Json(result))
}

async fn search_repo_modules_with_search_plane(
    state: Arc<GatewayState>,
    repo_id: &str,
    query: &str,
    limit: usize,
) -> Result<Option<crate::analyzers::ModuleSearchResult>, StudioApiError> {
    if !repo_entity_publication_ready(&state, repo_id).await {
        return Ok(None);
    }
    search_repo_entity_module_results(&state.studio.search_plane, repo_id, query, limit)
        .await
        .map(Some)
        .map_err(|error| {
            StudioApiError::internal(
                "REPO_MODULE_SEARCH_FAILED",
                "Repo module search task failed",
                Some(error.to_string()),
            )
        })
}

async fn search_repo_symbols_with_search_plane(
    state: Arc<GatewayState>,
    repo_id: &str,
    query: &str,
    limit: usize,
) -> Result<Option<crate::analyzers::SymbolSearchResult>, StudioApiError> {
    if !repo_entity_publication_ready(&state, repo_id).await {
        return Ok(None);
    }
    search_repo_entity_symbol_results(&state.studio.search_plane, repo_id, query, limit)
        .await
        .map(Some)
        .map_err(|error| {
            StudioApiError::internal(
                "REPO_SYMBOL_SEARCH_FAILED",
                "Repo symbol search task failed",
                Some(error.to_string()),
            )
        })
}

async fn search_repo_examples_with_search_plane(
    state: Arc<GatewayState>,
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

async fn repo_entity_publication_ready(state: &Arc<GatewayState>, repo_id: &str) -> bool {
    state
        .studio
        .search_plane
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, repo_id)
        .await
        .and_then(|record| record.publication)
        .is_some()
}

/// Doc coverage endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn doc_coverage(
    Query(query): Query<RepoDocCoverageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocCoverageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let module_id = query.module_id;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_DOC_COVERAGE_PANIC",
        "Repo doc coverage task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_doc_coverage(
                &DocCoverageQuery { repo_id, module_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}

/// Repo sync endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, the sync mode is invalid,
/// repository lookup fails, syncing fails, or the background task panics.
pub async fn sync(
    Query(query): Query<RepoSyncApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoSyncResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let mode = parse_repo_sync_mode(query.mode.as_deref())?;
    let result = with_repository(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_SYNC_PANIC",
        "Repo sync task failed unexpectedly",
        !matches!(mode, crate::analyzers::RepoSyncMode::Status),
        move |repository, cwd| {
            repo_sync_for_registered_repository(
                &RepoSyncQuery { repo_id, mode },
                &repository,
                cwd.as_path(),
            )
        },
    )
    .await?;
    Ok(Json(result))
}
