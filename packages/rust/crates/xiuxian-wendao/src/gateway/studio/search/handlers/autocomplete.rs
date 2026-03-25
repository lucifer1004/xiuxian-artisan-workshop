use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::AutocompleteResponse;
use crate::search_plane::SearchPlaneCacheTtl;

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
        state.studio.ensure_local_symbol_index_ready().await?;
        let cache_key = state
            .studio
            .search_plane
            .autocomplete_cache_key(prefix.as_str(), limit);
        if let Some(cache_key) = cache_key.as_ref()
            && let Some(cached) = state
                .studio
                .search_plane
                .cache_get_json::<Vec<crate::gateway::studio::types::AutocompleteSuggestion>>(
                    cache_key,
                )
                .await
        {
            return Ok(Json(AutocompleteResponse {
                prefix,
                suggestions: cached,
            }));
        }

        let suggestions = state
            .studio
            .autocomplete_local_symbols(prefix.as_str(), limit)
            .await?;
        if let Some(cache_key) = cache_key.as_ref() {
            state
                .studio
                .search_plane
                .cache_set_json(cache_key, SearchPlaneCacheTtl::Autocomplete, &suggestions)
                .await;
        }
        suggestions
    };

    Ok(Json(AutocompleteResponse {
        prefix,
        suggestions,
    }))
}
