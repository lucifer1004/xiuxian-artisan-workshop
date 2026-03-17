use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::link_graph::{LinkGraphDisplayHit, LinkGraphRetrievalMode, LinkGraphSearchOptions};

use super::router::{StudioApiError, StudioState};
use super::types::{
    AutocompleteResponse, AutocompleteSuggestion, AutocompleteSuggestionType, SearchHit,
    SearchResponse,
};

const DEFAULT_SEARCH_LIMIT: usize = 10;
const MAX_SEARCH_LIMIT: usize = 200;
const DEFAULT_AUTOCOMPLETE_LIMIT: usize = 5;
const MAX_AUTOCOMPLETE_LIMIT: usize = 20;

#[derive(Debug, Deserialize)]
pub(super) struct SearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AutocompleteQuery {
    prefix: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) async fn search_knowledge(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<StudioState>>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT);
    let index = state.graph_index().await?;
    let payload = index.search_planned_payload(raw_query, limit, LinkGraphSearchOptions::default());

    let hits = payload
        .hits
        .into_iter()
        .map(|hit| SearchHit {
            stem: hit.stem,
            title: strip_option(&hit.title),
            path: hit.path,
            doc_type: hit.doc_type,
            tags: hit.tags,
            score: hit.score.max(0.0),
            best_section: strip_option(&hit.best_section),
            match_reason: strip_option(&hit.match_reason),
        })
        .collect();

    Ok(Json(SearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count: payload.hit_count,
        graph_confidence_score: Some(payload.graph_confidence_score),
        selected_mode: Some(retrieval_mode_to_string(payload.selected_mode)),
    }))
}

pub(super) async fn search_autocomplete(
    Query(query): Query<AutocompleteQuery>,
    State(state): State<Arc<StudioState>>,
) -> Result<Json<AutocompleteResponse>, StudioApiError> {
    let prefix = query
        .prefix
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PREFIX", "`prefix` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AUTOCOMPLETE_LIMIT)
        .clamp(1, MAX_AUTOCOMPLETE_LIMIT);
    let index = state.graph_index().await?;
    let payload =
        index.search_planned_payload(prefix, limit.max(2), LinkGraphSearchOptions::default());

    Ok(Json(AutocompleteResponse {
        prefix: prefix.to_string(),
        suggestions: collect_autocomplete_suggestions(prefix, &payload.hits, limit),
    }))
}

fn retrieval_mode_to_string(mode: LinkGraphRetrievalMode) -> String {
    match mode {
        LinkGraphRetrievalMode::GraphOnly => "graph_only".to_string(),
        LinkGraphRetrievalMode::Hybrid => "hybrid".to_string(),
        LinkGraphRetrievalMode::VectorOnly => "vector_only".to_string(),
    }
}

fn strip_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

struct AutocompleteCollector<'a> {
    suggestions: Vec<AutocompleteSuggestion>,
    seen: HashSet<String>,
    prefix_lc: &'a str,
    limit: usize,
}

impl<'a> AutocompleteCollector<'a> {
    fn new(prefix_lc: &'a str, limit: usize) -> Self {
        Self {
            suggestions: Vec::with_capacity(limit),
            seen: HashSet::new(),
            prefix_lc,
            limit,
        }
    }

    fn add(
        &mut self,
        text: &str,
        path: &str,
        doc_type: Option<&str>,
        suggestion_type: AutocompleteSuggestionType,
    ) {
        if self.suggestions.len() >= self.limit {
            return;
        }

        let normalized_text = text.trim();
        if normalized_text.is_empty()
            || !normalized_text
                .to_ascii_lowercase()
                .starts_with(self.prefix_lc)
        {
            return;
        }

        let key = format!("{suggestion_type:?}|{normalized_text}|{path}");
        if !self.seen.insert(key) {
            return;
        }

        self.suggestions.push(AutocompleteSuggestion {
            text: normalized_text.to_string(),
            suggestion_type,
            path: Some(path.to_string()),
            doc_type: doc_type.map(ToString::to_string),
        });
    }
}

fn collect_autocomplete_suggestions(
    prefix: &str,
    hits: &[LinkGraphDisplayHit],
    limit: usize,
) -> Vec<AutocompleteSuggestion> {
    let prefix_lc = prefix.to_ascii_lowercase();
    let mut collector = AutocompleteCollector::new(&prefix_lc, limit);

    for hit in hits {
        collector.add(
            &hit.stem,
            hit.path.as_str(),
            hit.doc_type.as_deref(),
            AutocompleteSuggestionType::Stem,
        );

        if !hit.title.is_empty() {
            collector.add(
                &hit.title,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Title,
            );
        }

        for tag in &hit.tags {
            collector.add(
                tag,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Tag,
            );
        }

        if collector.suggestions.len() >= limit {
            break;
        }
    }

    collector.suggestions
}

#[cfg(test)]
#[path = "../../../tests/unit/gateway/studio/search.rs"]
mod tests;
