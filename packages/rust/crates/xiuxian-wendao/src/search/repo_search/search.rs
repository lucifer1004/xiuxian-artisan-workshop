use xiuxian_vector::LanceRecordBatch;
use xiuxian_wendao_runtime::transport::RepoSearchFlightRequest;

use super::batch::repo_search_batch_from_hits;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::{RepoContentChunkSearchFilters, SearchPlaneService};

pub(crate) async fn search_repo_content_hits_for_query(
    search_plane: &SearchPlaneService,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let parsed = super::query::parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    if !parsed.kind_filters.is_empty() && !parsed.kind_filters.contains("file") {
        return Ok(Vec::new());
    }

    search_repo_content_hits(
        search_plane,
        &RepoSearchFlightRequest {
            repo_id: repo_id.to_string(),
            query_text: search_term.to_string(),
            limit,
            language_filters: parsed.language_filters.clone(),
            path_prefixes: std::collections::HashSet::new(),
            title_filters: std::collections::HashSet::new(),
            tag_filters: std::collections::HashSet::new(),
            filename_filters: std::collections::HashSet::new(),
        },
    )
    .await
}

pub(crate) async fn search_repo_content_hits(
    search_plane: &SearchPlaneService,
    request: &RepoSearchFlightRequest,
) -> Result<Vec<SearchHit>, String> {
    let repo_id = request.repo_id.trim();
    if repo_id.is_empty() {
        return Err("repo-search request repo_id must not be blank".to_string());
    }

    search_plane
        .search_repo_content_chunks_with_filters(
            repo_id,
            request.query_text.as_str(),
            &request.language_filters,
            &RepoContentChunkSearchFilters {
                path_prefixes: request.path_prefixes.clone(),
                filename_filters: request.filename_filters.clone(),
                title_filters: request.title_filters.clone(),
                tag_filters: request.tag_filters.clone(),
            },
            request.limit,
        )
        .await
        .map_err(|error| format!("repo-search content query failed for repo `{repo_id}`: {error}"))
}

pub(crate) async fn search_repo_content_batch(
    search_plane: &SearchPlaneService,
    request: &RepoSearchFlightRequest,
) -> Result<LanceRecordBatch, String> {
    let hits = search_repo_content_hits(search_plane, request).await?;
    repo_search_batch_from_hits(&hits)
}
