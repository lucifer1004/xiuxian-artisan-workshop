use std::collections::HashSet;

use crate::gateway::studio::types::SearchHit;
use crate::search_plane::ranking::sort_by_rank;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::RepoContentChunkSearchError;
use super::compare_candidates;
use super::execution::execute_repo_content_search;
use super::retained_window;
use super::scan::build_repo_content_scan_options;

pub(crate) async fn search_repo_content_chunks(
    service: &SearchPlaneService,
    repo_id: &str,
    search_term: &str,
    language_filters: &HashSet<String>,
    limit: usize,
) -> Result<Vec<SearchHit>, RepoContentChunkSearchError> {
    let trimmed = search_term.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let _read_permit = service.acquire_repo_search_read_permit().await?;
    let store = service
        .open_store(SearchCorpusKind::RepoContentChunk)
        .await?;
    let table_name = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
        .await
        .and_then(|record| record.publication.map(|publication| publication.table_name))
        .unwrap_or_else(|| SearchPlaneService::repo_content_chunk_table_name(repo_id));
    if !store.table_path(table_name.as_str()).exists() {
        return Ok(Vec::new());
    }

    let options = build_repo_content_scan_options(language_filters, trimmed, limit);
    let needle = trimmed.to_ascii_lowercase();
    let execution = execute_repo_content_search(
        &store,
        table_name.as_str(),
        trimmed,
        needle.as_str(),
        options,
        retained_window(limit),
    )
    .await?;
    let mut hits = execution.candidates;
    sort_by_rank(&mut hits, compare_candidates);
    hits.truncate(limit);
    let hits = hits
        .into_iter()
        .map(|candidate| candidate.into_search_hit(repo_id))
        .collect::<Vec<_>>();
    service.record_query_telemetry(
        SearchCorpusKind::RepoContentChunk,
        execution
            .telemetry
            .finish(execution.source, Some(repo_id.to_string()), hits.len()),
    );
    Ok(hits)
}
