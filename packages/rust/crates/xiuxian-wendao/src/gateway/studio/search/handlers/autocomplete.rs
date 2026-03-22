use std::collections::HashSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{AstSearchHit, AutocompleteResponse, AutocompleteSuggestion};

use super::queries::AutocompleteQuery;

pub async fn search_autocomplete(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AutocompleteQuery>,
) -> Result<Json<AutocompleteResponse>, StudioApiError> {
    let prefix = query.prefix.unwrap_or_default().trim().to_string();
    let limit = query.limit.unwrap_or(8).max(1);
    let suggestions = if prefix.is_empty() {
        Vec::new()
    } else {
        let ast_index = state.studio.ast_index().await?;
        build_autocomplete_suggestions(ast_index.as_slice(), prefix.as_str(), limit)
    };

    Ok(Json(AutocompleteResponse {
        prefix,
        suggestions,
    }))
}

fn build_autocomplete_suggestions(
    ast_hits: &[AstSearchHit],
    prefix: &str,
    limit: usize,
) -> Vec<AutocompleteSuggestion> {
    let normalized_prefix = prefix.to_ascii_lowercase();
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();

    for suggestion in ast_hits
        .iter()
        .filter_map(|hit| autocomplete_suggestion_from_ast(hit, normalized_prefix.as_str()))
    {
        let dedupe_key = suggestion.text.to_ascii_lowercase();
        if seen.insert(dedupe_key) {
            suggestions.push(suggestion);
        }
    }

    suggestions.sort_by(|left, right| {
        autocomplete_suggestion_rank(left)
            .cmp(&autocomplete_suggestion_rank(right))
            .then_with(|| left.text.cmp(&right.text))
    });
    suggestions.truncate(limit);
    suggestions
}

fn autocomplete_suggestion_from_ast(
    hit: &AstSearchHit,
    normalized_prefix: &str,
) -> Option<AutocompleteSuggestion> {
    let text = hit.name.trim();
    if text.is_empty() || !autocomplete_matches_prefix(text, normalized_prefix) {
        return None;
    }

    Some(AutocompleteSuggestion {
        text: text.to_string(),
        suggestion_type: autocomplete_suggestion_type(hit).to_string(),
    })
}

fn autocomplete_matches_prefix(text: &str, normalized_prefix: &str) -> bool {
    let normalized_text = text.to_ascii_lowercase();
    if normalized_text.starts_with(normalized_prefix) {
        return true;
    }

    normalized_text
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| !token.is_empty() && token.starts_with(normalized_prefix))
}

fn autocomplete_suggestion_type(hit: &AstSearchHit) -> &'static str {
    if hit.language == "markdown" {
        match hit.node_kind.as_deref() {
            Some("property" | "observation") => "metadata",
            _ => "heading",
        }
    } else {
        "symbol"
    }
}

fn autocomplete_suggestion_rank(suggestion: &AutocompleteSuggestion) -> usize {
    match suggestion.suggestion_type.as_str() {
        "symbol" => 0,
        "heading" => 1,
        "metadata" => 2,
        _ => 3,
    }
}
