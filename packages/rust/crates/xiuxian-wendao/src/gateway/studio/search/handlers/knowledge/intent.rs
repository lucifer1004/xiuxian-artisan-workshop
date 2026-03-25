use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use super::helpers::{
    compare_intent_hits, intent_candidate_limit, is_code_biased_intent, is_index_not_ready,
    is_ui_config_required, local_symbol_hit_to_search_hit, repo_content_hit_to_intent_hit,
};
use super::merge::{RepoIntentMerge, build_repo_intent_merge};
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::search::handlers::code_search::search::build_code_search_response;
use crate::gateway::studio::search::handlers::queries::SearchQuery;
use crate::gateway::studio::types::{AstSearchHit, SearchHit, SearchResponse};
use crate::search_plane::{RepoSearchQueryCacheKeyInput, SearchCorpusKind, SearchPlaneCacheTtl};

struct IntentIndexState {
    knowledge_config_missing: bool,
    symbol_config_missing: bool,
}

struct IntentSourceHits {
    knowledge_hits: Vec<SearchHit>,
    local_symbol_hits: Vec<AstSearchHit>,
    knowledge_indexing: bool,
    local_symbol_indexing: bool,
}

struct IntentMergedResults {
    hits: Vec<SearchHit>,
    knowledge_hit_count: usize,
    local_symbol_hit_count: usize,
    repo_hit_count: usize,
    partial: bool,
    pending_repos: Vec<String>,
    skipped_repos: Vec<String>,
}

pub async fn search_intent(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    let intent = query.intent.clone().unwrap_or_default();
    let limit = query.limit.unwrap_or(10).max(1);

    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Intent search requires a non-empty query",
        ));
    }

    if intent == "code_search" {
        let response = build_code_search_response(
            state.studio.as_ref(),
            raw_query,
            query.repo.as_deref(),
            limit,
        )
        .await?;
        return Ok(Json(response));
    }

    let response = build_intent_search_response(
        state.studio.as_ref(),
        raw_query.as_str(),
        query_text,
        query.repo.as_deref(),
        limit,
        (!intent.is_empty()).then_some(intent),
    )
    .await?;
    Ok(Json(response))
}

pub async fn build_intent_search_response(
    studio: &StudioState,
    raw_query: &str,
    query_text: &str,
    repo_hint: Option<&str>,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchResponse, StudioApiError> {
    let index_state = ensure_intent_indices(studio)?;
    let candidate_limit = intent_candidate_limit(limit);
    let intent_ref = intent.as_deref();
    let code_biased = is_code_biased_intent(intent_ref, query_text, repo_hint);
    let cache_key = build_intent_cache_key(
        studio,
        raw_query,
        query_text,
        repo_hint,
        limit,
        intent_ref,
        code_biased,
    )
    .await?;
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let source_hits =
        search_intent_sources(studio, query_text, candidate_limit, &index_state).await?;

    let repo_merge = if code_biased {
        build_repo_intent_merge(studio, raw_query, repo_hint, candidate_limit).await?
    } else {
        RepoIntentMerge::default()
    };

    let merged = merge_intent_hits(source_hits, repo_merge, code_biased);
    if missing_intent_config(&index_state, &merged) {
        return Err(missing_intent_config_error());
    }

    let response = build_intent_response(query_text, limit, intent, merged);
    if !response.partial
        && let Some(cache_key) = cache_key.as_ref()
    {
        studio
            .search_plane
            .cache_set_json(cache_key, SearchPlaneCacheTtl::HotQuery, &response)
            .await;
    }
    Ok(response)
}

fn ensure_intent_indices(studio: &StudioState) -> Result<IntentIndexState, StudioApiError> {
    let knowledge_start = studio.ensure_knowledge_section_index_started();
    let symbol_start = studio.ensure_local_symbol_index_started();
    let knowledge_config_missing =
        matches!(knowledge_start, Err(ref error) if is_ui_config_required(error));
    let symbol_config_missing =
        matches!(symbol_start, Err(ref error) if is_ui_config_required(error));
    if let Err(error) = knowledge_start.as_ref()
        && !is_ui_config_required(error)
    {
        return Err(error.clone());
    }
    if let Err(error) = symbol_start.as_ref()
        && !is_ui_config_required(error)
    {
        return Err(error.clone());
    }
    Ok(IntentIndexState {
        knowledge_config_missing,
        symbol_config_missing,
    })
}

async fn build_intent_cache_key(
    studio: &StudioState,
    raw_query: &str,
    query_text: &str,
    repo_hint: Option<&str>,
    limit: usize,
    intent: Option<&str>,
    code_biased: bool,
) -> Result<Option<String>, StudioApiError> {
    if code_biased {
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
        return Ok(studio
            .search_plane
            .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
                scope: "intent_hybrid_code",
                corpora: &[
                    SearchCorpusKind::KnowledgeSection,
                    SearchCorpusKind::LocalSymbol,
                ],
                repo_corpora: &[
                    SearchCorpusKind::RepoEntity,
                    SearchCorpusKind::RepoContentChunk,
                ],
                repo_ids: repo_ids.as_slice(),
                query: raw_query,
                limit,
                intent,
                repo_hint,
            })
            .await);
    }

    Ok(studio.search_plane.search_query_cache_key(
        "intent_hybrid",
        &[
            SearchCorpusKind::KnowledgeSection,
            SearchCorpusKind::LocalSymbol,
        ],
        query_text,
        limit,
        intent,
        None,
    ))
}

fn merge_intent_hits(
    source_hits: IntentSourceHits,
    repo_merge: RepoIntentMerge,
    code_biased: bool,
) -> IntentMergedResults {
    let mut hits = Vec::new();
    let knowledge_hit_count = source_hits.knowledge_hits.len();
    hits.extend(source_hits.knowledge_hits);

    let local_symbol_hit_count = source_hits.local_symbol_hits.len();
    hits.extend(
        source_hits
            .local_symbol_hits
            .into_iter()
            .map(|hit| local_symbol_hit_to_search_hit(hit, code_biased)),
    );

    let repo_hit_count = repo_merge.hits.len();
    hits.extend(
        repo_merge
            .hits
            .into_iter()
            .map(|hit| repo_content_hit_to_intent_hit(hit, code_biased)),
    );

    IntentMergedResults {
        hits,
        knowledge_hit_count,
        local_symbol_hit_count,
        repo_hit_count,
        partial: source_hits.knowledge_indexing
            || source_hits.local_symbol_indexing
            || !repo_merge.pending_repos.is_empty()
            || !repo_merge.skipped_repos.is_empty(),
        pending_repos: repo_merge.pending_repos,
        skipped_repos: repo_merge.skipped_repos,
    }
}

fn missing_intent_config(index_state: &IntentIndexState, merged: &IntentMergedResults) -> bool {
    merged.hits.is_empty()
        && merged.pending_repos.is_empty()
        && merged.skipped_repos.is_empty()
        && index_state.knowledge_config_missing
        && index_state.symbol_config_missing
}

fn build_intent_response(
    query_text: &str,
    limit: usize,
    intent: Option<String>,
    mut merged: IntentMergedResults,
) -> SearchResponse {
    merged.hits.sort_by(compare_intent_hits);
    merged.hits.truncate(limit);

    let selected_mode = if merged.hits.is_empty() {
        "vector_only".to_string()
    } else if merged.local_symbol_hit_count > 0 || merged.repo_hit_count > 0 {
        "intent_hybrid".to_string()
    } else {
        "graph_fts".to_string()
    };
    let indexing_state = if merged.partial {
        Some(if merged.hits.is_empty() {
            "indexing".to_string()
        } else {
            "partial".to_string()
        })
    } else {
        None
    };

    SearchResponse {
        query: query_text.to_string(),
        hit_count: merged.hits.len(),
        hits: merged.hits,
        graph_confidence_score: Some(if merged.knowledge_hit_count > 0 {
            1.0
        } else {
            0.0
        }),
        selected_mode: Some(selected_mode.clone()),
        intent,
        intent_confidence: Some(if selected_mode == "vector_only" {
            0.0
        } else {
            1.0
        }),
        search_mode: Some(selected_mode),
        partial: merged.partial,
        indexing_state,
        pending_repos: merged.pending_repos,
        skipped_repos: merged.skipped_repos,
    }
}

async fn search_intent_sources(
    studio: &StudioState,
    query_text: &str,
    candidate_limit: usize,
    index_state: &IntentIndexState,
) -> Result<IntentSourceHits, StudioApiError> {
    let (knowledge_result, symbol_result) = tokio::join!(
        async {
            if index_state.knowledge_config_missing {
                Ok(Vec::new())
            } else {
                studio
                    .search_knowledge_sections(query_text, candidate_limit)
                    .await
            }
        },
        async {
            if index_state.symbol_config_missing {
                Ok(Vec::new())
            } else {
                studio
                    .search_local_symbol_hits(query_text, candidate_limit)
                    .await
            }
        }
    );

    let (knowledge_hits, knowledge_indexing) = decode_intent_source_result(knowledge_result)?;
    let (local_symbol_hits, local_symbol_indexing) = decode_intent_source_result(symbol_result)?;
    Ok(IntentSourceHits {
        knowledge_hits,
        local_symbol_hits,
        knowledge_indexing,
        local_symbol_indexing,
    })
}

fn decode_intent_source_result<T>(
    result: Result<Vec<T>, StudioApiError>,
) -> Result<(Vec<T>, bool), StudioApiError> {
    match result {
        Ok(hits) => Ok((hits, false)),
        Err(error) if is_index_not_ready(&error) => Ok((Vec::new(), true)),
        Err(error) => Err(error),
    }
}

fn missing_intent_config_error() -> StudioApiError {
    StudioApiError::bad_request(
        "UI_CONFIG_REQUIRED",
        "Studio intent search requires configured link_graph.projects or repo_projects",
    )
}
