use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use super::code_search::{
    build_code_search_response, build_repo_content_search_hits, build_repo_entity_search_hits,
};
use super::queries::SearchQuery;
use crate::gateway::studio::repo_index::RepoIndexPhase;
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::types::{AstSearchHit, SearchHit, SearchResponse};
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

async fn build_knowledge_search_response(
    studio: &StudioState,
    query_text: &str,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchResponse, StudioApiError> {
    studio.ensure_knowledge_section_index_started()?;
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

pub(super) async fn build_intent_search_response(
    studio: &StudioState,
    raw_query: &str,
    query_text: &str,
    repo_hint: Option<&str>,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchResponse, StudioApiError> {
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

    let candidate_limit = intent_candidate_limit(limit);
    let code_biased = is_code_biased_intent(intent.as_deref(), query_text, repo_hint);
    let cache_key = if code_biased {
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
        let repo_status = studio.repo_index.status_response(repo_hint);
        studio
            .search_plane
            .repo_search_query_cache_key(
                "intent_hybrid_code",
                &[
                    SearchCorpusKind::KnowledgeSection,
                    SearchCorpusKind::LocalSymbol,
                ],
                &[
                    SearchCorpusKind::RepoEntity,
                    SearchCorpusKind::RepoContentChunk,
                ],
                &repo_status,
                repo_ids.as_slice(),
                raw_query,
                limit,
                intent.as_deref(),
                repo_hint,
            )
            .await
    } else {
        studio.search_plane.search_query_cache_key(
            "intent_hybrid",
            &[
                SearchCorpusKind::KnowledgeSection,
                SearchCorpusKind::LocalSymbol,
            ],
            query_text,
            limit,
            intent.as_deref(),
            None,
        )
    };
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let (knowledge_result, symbol_result) = tokio::join!(
        async {
            if knowledge_config_missing {
                Ok(Vec::new())
            } else {
                studio
                    .search_knowledge_sections(query_text, candidate_limit)
                    .await
            }
        },
        async {
            if symbol_config_missing {
                Ok(Vec::new())
            } else {
                studio
                    .search_local_symbol_hits(query_text, candidate_limit)
                    .await
            }
        }
    );

    let mut knowledge_indexing = false;
    let knowledge_hits = match knowledge_result {
        Ok(hits) => hits,
        Err(error) if is_index_not_ready(&error) => {
            knowledge_indexing = true;
            Vec::new()
        }
        Err(error) => return Err(error),
    };
    let mut local_symbol_indexing = false;
    let local_symbol_hits = match symbol_result {
        Ok(hits) => hits,
        Err(error) if is_index_not_ready(&error) => {
            local_symbol_indexing = true;
            Vec::new()
        }
        Err(error) => return Err(error),
    };

    let repo_merge = if code_biased {
        build_repo_intent_merge(studio, raw_query, repo_hint, candidate_limit).await?
    } else {
        RepoIntentMerge::default()
    };

    let mut hits = Vec::new();
    let knowledge_hit_count = knowledge_hits.len();
    hits.extend(knowledge_hits);
    let local_symbol_hit_count = local_symbol_hits.len();
    hits.extend(
        local_symbol_hits
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

    if hits.is_empty()
        && repo_merge.pending_repos.is_empty()
        && repo_merge.skipped_repos.is_empty()
        && knowledge_config_missing
        && symbol_config_missing
    {
        return Err(knowledge_start
            .err()
            .or(symbol_start.err())
            .unwrap_or_else(|| {
                StudioApiError::bad_request(
                    "UI_CONFIG_REQUIRED",
                    "Studio intent search requires configured link_graph.projects or repo_projects",
                )
            }));
    }

    hits.sort_by(compare_intent_hits);
    hits.truncate(limit);

    let partial = knowledge_indexing
        || local_symbol_indexing
        || !repo_merge.pending_repos.is_empty()
        || !repo_merge.skipped_repos.is_empty();
    let selected_mode = if hits.is_empty() {
        "vector_only".to_string()
    } else if local_symbol_hit_count > 0 || repo_hit_count > 0 {
        "intent_hybrid".to_string()
    } else {
        "graph_fts".to_string()
    };
    let indexing_state = if partial {
        Some(if hits.is_empty() {
            "indexing".to_string()
        } else {
            "partial".to_string()
        })
    } else {
        None
    };
    let graph_confidence_score = if knowledge_hit_count > 0 { 1.0 } else { 0.0 };
    let intent_confidence = if hits.is_empty() { 0.0 } else { 1.0 };

    let response = SearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        graph_confidence_score: Some(graph_confidence_score),
        selected_mode: Some(selected_mode.clone()),
        intent,
        intent_confidence: Some(intent_confidence),
        search_mode: Some(selected_mode),
        partial,
        indexing_state,
        pending_repos: repo_merge.pending_repos,
        skipped_repos: repo_merge.skipped_repos,
    };
    if !response.partial {
        if let Some(cache_key) = cache_key.as_ref() {
            studio
                .search_plane
                .cache_set_json(cache_key, SearchPlaneCacheTtl::HotQuery, &response)
                .await;
        }
    }
    Ok(response)
}

#[derive(Debug, Default)]
struct RepoIntentMerge {
    hits: Vec<SearchHit>,
    pending_repos: Vec<String>,
    skipped_repos: Vec<String>,
}

async fn build_repo_intent_merge(
    studio: &StudioState,
    raw_query: &str,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<RepoIntentMerge, StudioApiError> {
    let repositories = if let Some(repo_id) = repo_hint {
        vec![configured_repository(studio, repo_id).map_err(map_repo_intelligence_error)?]
    } else {
        configured_repositories(studio)
    };

    let mut merge = RepoIntentMerge::default();
    for repository in repositories {
        let has_repo_entity_publication = studio
            .search_plane
            .has_published_repo_corpus(SearchCorpusKind::RepoEntity, repository.id.as_str())
            .await;
        let has_repo_content_publication = studio
            .search_plane
            .has_published_repo_corpus(SearchCorpusKind::RepoContentChunk, repository.id.as_str())
            .await;
        if !has_repo_entity_publication && !has_repo_content_publication {
            let repo_status = studio
                .repo_index
                .status_response(Some(repository.id.as_str()));
            let phase = repo_status.repos.first().map(|status| status.phase);
            if matches!(
                phase,
                Some(RepoIndexPhase::Unsupported | RepoIndexPhase::Failed)
            ) {
                merge.skipped_repos.push(repository.id.clone());
            } else {
                merge.pending_repos.push(repository.id.clone());
            }
            continue;
        }
        if has_repo_entity_publication {
            merge.hits.extend(
                build_repo_entity_search_hits(studio, repository.id.as_str(), raw_query, limit)
                    .await?,
            );
        }
        if has_repo_content_publication {
            merge.hits.extend(
                build_repo_content_search_hits(studio, repository.id.as_str(), raw_query, limit)
                    .await?,
            );
        }
    }

    Ok(merge)
}

fn local_symbol_hit_to_search_hit(hit: AstSearchHit, code_biased: bool) -> SearchHit {
    let mut tags = vec![
        hit.crate_name.clone(),
        "code".to_string(),
        "symbol".to_string(),
        hit.language.clone(),
        format!("lang:{}", hit.language),
    ];
    if let Some(node_kind) = hit.node_kind.as_deref() {
        tags.push(node_kind.to_string());
        tags.push(format!("kind:{node_kind}"));
    } else {
        tags.push("kind:symbol".to_string());
    }
    if let Some(project_name) = hit.project_name.as_deref() {
        tags.push(project_name.to_string());
    }

    let best_section = if !hit.signature.trim().is_empty() {
        Some(hit.signature.clone())
    } else {
        hit.owner_title.clone()
    };

    SearchHit {
        stem: hit.name.clone(),
        title: Some(hit.name),
        path: hit.path.clone(),
        doc_type: Some("symbol".to_string()),
        tags,
        score: normalize_local_symbol_score(hit.score, code_biased),
        best_section,
        match_reason: Some("local_symbol_search".to_string()),
        hierarchical_uri: None,
        hierarchy: Some(hit.path.split('/').map(str::to_string).collect::<Vec<_>>()),
        saliency_score: None,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: None,
        implicit_backlink_items: None,
        navigation_target: Some(hit.navigation_target),
    }
}

fn repo_content_hit_to_intent_hit(mut hit: SearchHit, code_biased: bool) -> SearchHit {
    if code_biased {
        hit.score = (hit.score + 0.04).min(0.9);
    }
    hit
}

fn normalize_local_symbol_score(score: f64, code_biased: bool) -> f64 {
    if code_biased {
        (score + 0.02).min(1.0)
    } else {
        score
    }
}

fn compare_intent_hits(left: &SearchHit, right: &SearchHit) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| intent_hit_priority(right).cmp(&intent_hit_priority(left)))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.stem.cmp(&right.stem))
}

fn intent_hit_priority(hit: &SearchHit) -> u8 {
    match hit.doc_type.as_deref() {
        Some("symbol") => 3,
        Some("file") => 2,
        _ => 1,
    }
}

fn intent_candidate_limit(limit: usize) -> usize {
    limit.saturating_mul(2).max(8)
}

fn is_code_biased_intent(intent: Option<&str>, query_text: &str, repo_hint: Option<&str>) -> bool {
    if repo_hint.is_some() {
        return true;
    }

    let normalized = intent.unwrap_or_default().to_ascii_lowercase();
    if [
        "code",
        "debug",
        "symbol",
        "definition",
        "reference",
        "implement",
        "trace",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
    {
        return true;
    }

    query_text.contains("lang:")
        || query_text.contains("kind:")
        || query_text.contains("repo:")
        || query_text
            .chars()
            .any(|ch| matches!(ch, '_' | ':' | '(' | ')' | '/' | '@'))
}

fn is_index_not_ready(error: &StudioApiError) -> bool {
    error.status() == axum::http::StatusCode::CONFLICT && error.code() == "INDEX_NOT_READY"
}

fn is_ui_config_required(error: &StudioApiError) -> bool {
    error.status() == axum::http::StatusCode::BAD_REQUEST && error.code() == "UI_CONFIG_REQUIRED"
}
