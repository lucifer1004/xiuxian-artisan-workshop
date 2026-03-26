use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;

use crate::gateway::studio::search::handlers::code_search::query::parse_repo_code_search_query;

/// Search repo entity rows for a repo-scoped code query.
///
/// # Errors
///
/// Returns [`StudioApiError`] when the repo entity search plane fails.
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

/// Search repo content rows for a repo-scoped code query.
///
/// # Errors
///
/// Returns [`StudioApiError`] when the repo content search plane fails.
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
use crate::gateway::studio::router::StudioState;

#[cfg(test)]
/// Build repo entity search hits through the Studio state wrapper.
pub(crate) async fn build_repo_entity_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_entity_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

#[cfg(test)]
/// Build repo content search hits through the Studio state wrapper.
pub(crate) async fn build_repo_content_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_content_hits(&studio.search_plane, repo_id, raw_query, limit).await
}
