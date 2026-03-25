use xiuxian_vector::ColumnarScanOptions;

use crate::gateway::studio::types::AstSearchHit;
use crate::search_plane::local_symbol::query::shared::{
    LocalSymbolSearchError, compare_candidates, decode_local_symbol_hits,
    execute_local_symbol_search, retained_window,
};
use crate::search_plane::local_symbol::schema::{hit_json_column, projected_columns};
use crate::search_plane::ranking::sort_by_rank;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

pub(crate) async fn search_local_symbols(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
) -> Result<Vec<AstSearchHit>, LocalSymbolSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    let Some(active_epoch) = status.active_epoch else {
        return Err(LocalSymbolSearchError::NotReady);
    };
    let query_lower = query.trim().to_ascii_lowercase();
    if query_lower.is_empty() {
        return Ok(Vec::new());
    }

    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if table_names.is_empty() {
        return Ok(Vec::new());
    }
    let mut columns = projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    columns.push(hit_json_column().to_string());
    let window = retained_window(limit);
    let execution = execute_local_symbol_search(
        &store,
        table_names.as_slice(),
        query_lower.as_str(),
        ColumnarScanOptions {
            projected_columns: columns,
            batch_size: Some(256),
            limit: Some(limit.saturating_mul(32).max(128)),
            ..ColumnarScanOptions::default()
        },
        window,
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits = decode_local_symbol_hits(candidates)?;
    service.record_query_telemetry(
        SearchCorpusKind::LocalSymbol,
        execution
            .telemetry
            .finish(execution.source, Some("search".to_string()), hits.len()),
    );
    Ok(hits)
}
