use xiuxian_vector::ColumnarScanOptions;

use crate::gateway::studio::types::AutocompleteSuggestion;
use crate::search_plane::local_symbol::query::shared::{
    LocalSymbolSearchError, compare_suggestions, execute_local_symbol_autocomplete,
    suggestion_window,
};
use crate::search_plane::local_symbol::schema::suggestion_columns;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

pub(crate) async fn autocomplete_local_symbols(
    service: &SearchPlaneService,
    prefix: &str,
    limit: usize,
) -> Result<Vec<AutocompleteSuggestion>, LocalSymbolSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    let Some(active_epoch) = status.active_epoch else {
        return Err(LocalSymbolSearchError::NotReady);
    };

    let normalized_prefix = prefix.trim().to_ascii_lowercase();
    if normalized_prefix.is_empty() {
        return Ok(Vec::new());
    }

    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if table_names.is_empty() {
        return Ok(Vec::new());
    }
    let execution = execute_local_symbol_autocomplete(
        &store,
        table_names.as_slice(),
        normalized_prefix.as_str(),
        ColumnarScanOptions {
            projected_columns: suggestion_columns()
                .into_iter()
                .map(str::to_string)
                .collect(),
            batch_size: Some(256),
            limit: Some(limit.saturating_mul(64).max(256)),
            ..ColumnarScanOptions::default()
        },
        suggestion_window(limit),
    )
    .await?;
    let mut suggestions = execution.suggestions;
    suggestions.sort_by(|left, right| compare_suggestions(left, right));
    suggestions.truncate(limit);
    service.record_query_telemetry(
        SearchCorpusKind::LocalSymbol,
        execution.telemetry.finish(
            execution.source,
            Some("autocomplete".to_string()),
            suggestions.len(),
        ),
    );
    Ok(suggestions)
}
