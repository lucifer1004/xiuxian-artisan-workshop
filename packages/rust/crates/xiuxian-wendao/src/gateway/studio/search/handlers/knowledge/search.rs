use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use super::super::queries::SearchQuery;
use crate::gateway::studio::router::{GatewayState, StudioApiError, StudioState};
use crate::gateway::studio::types::SearchResponse;
use crate::search_plane::{SearchCorpusKind, SearchPlaneCacheTtl};

pub async fn search_knowledge(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Knowledge search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(10).max(1);
    let response = build_knowledge_search_response(
        state.studio.as_ref(),
        query_text,
        limit,
        query
            .intent
            .clone()
            .or_else(|| Some("semantic_lookup".to_string())),
    )
    .await?;
    Ok(Json(response))
}

async fn build_knowledge_search_response(
    studio: &StudioState,
    query_text: &str,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchResponse, StudioApiError> {
    studio.ensure_knowledge_section_index_ready().await?;
    let cache_key = studio.search_plane.search_query_cache_key(
        "knowledge",
        &[SearchCorpusKind::KnowledgeSection],
        query_text,
        limit,
        intent.as_deref(),
        None,
    );
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let hits = studio.search_knowledge_sections(query_text, limit).await?;

    let selected_mode = if hits.is_empty() {
        "vector_only".to_string()
    } else {
        "graph_fts".to_string()
    };
    let graph_confidence_score = if hits.is_empty() { 0.0 } else { 1.0 };
    let response = SearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        graph_confidence_score: Some(graph_confidence_score),
        selected_mode: Some(selected_mode.clone()),
        intent,
        intent_confidence: Some(graph_confidence_score),
        search_mode: Some(selected_mode),
        partial: false,
        indexing_state: None,
        pending_repos: Vec::new(),
        skipped_repos: Vec::new(),
    };
    if let Some(cache_key) = cache_key.as_ref() {
        studio
            .search_plane
            .cache_set_json(cache_key, SearchPlaneCacheTtl::HotQuery, &response)
            .await;
    }
    Ok(response)
}
