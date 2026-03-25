use std::collections::HashSet;

use crate::search_plane::SearchPlaneService;
use crate::search_plane::repo_entity::query::execution::execute_repo_entity_search;
use crate::search_plane::repo_entity::query::types::{
    PreparedRepoEntitySearch, RepoEntityQuery, RepoEntitySearchError,
};
use crate::search_plane::repo_entity::schema::projected_columns;
use xiuxian_vector::ColumnarScanOptions;

pub(crate) async fn prepare_repo_entity_search(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    language_filters: &HashSet<String>,
    kind_filters: &HashSet<String>,
    limit: usize,
) -> Result<Option<PreparedRepoEntitySearch>, RepoEntitySearchError> {
    let trimmed = query.trim();
    let query_lower = trimmed.to_ascii_lowercase();
    let read_permit = service.acquire_repo_search_read_permit().await?;
    let store = service
        .open_store(crate::search_plane::SearchCorpusKind::RepoEntity)
        .await?;
    let table_name = service
        .repo_corpus_record_for_reads(crate::search_plane::SearchCorpusKind::RepoEntity, repo_id)
        .await
        .and_then(|record| record.publication.map(|publication| publication.table_name))
        .unwrap_or_else(|| SearchPlaneService::repo_entity_table_name(repo_id));
    if !store.table_path(table_name.as_str()).exists() {
        return Ok(None);
    }

    let columns = projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let options = build_repo_entity_scan_options(language_filters, trimmed, limit, columns);
    let query = RepoEntityQuery {
        query_text: trimmed,
        query_lower: query_lower.as_str(),
        language_filters,
        kind_filters,
        window: crate::search_plane::repo_entity::query::execution::retained_window(limit),
    };
    let execution =
        execute_repo_entity_search(&store, table_name.as_str(), options, &query).await?;
    let mut candidates = execution.candidates;
    crate::search_plane::ranking::sort_by_rank(
        &mut candidates,
        crate::search_plane::repo_entity::query::execution::compare_candidates,
    );
    candidates.truncate(limit);

    Ok(Some(PreparedRepoEntitySearch {
        _read_permit: read_permit,
        store,
        table_name,
        candidates,
        telemetry: execution.telemetry,
        source: execution.source,
    }))
}

fn build_repo_entity_scan_options(
    language_filters: &HashSet<String>,
    trimmed: &str,
    limit: usize,
    projected_columns: Vec<String>,
) -> ColumnarScanOptions {
    ColumnarScanOptions {
        where_filter: filter_expression(language_filters),
        projected_columns,
        batch_size: Some(512),
        limit: if should_use_fts(trimmed) {
            Some(limit.saturating_mul(32).max(128))
        } else {
            None
        },
        ..ColumnarScanOptions::default()
    }
}

fn should_use_fts(query: &str) -> bool {
    query.chars().any(char::is_alphanumeric) && query.len() >= 2
}

fn filter_expression(language_filters: &HashSet<String>) -> Option<String> {
    if language_filters.is_empty() {
        return None;
    }

    let mut sorted = language_filters.iter().cloned().collect::<Vec<_>>();
    sorted.sort_unstable();
    Some(
        sorted
            .into_iter()
            .map(|value| format!("language = '{}'", value.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}
