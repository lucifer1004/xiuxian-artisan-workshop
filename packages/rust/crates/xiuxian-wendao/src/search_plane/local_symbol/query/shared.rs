use std::collections::HashSet;

use xiuxian_vector::{
    LanceArray, LanceRecordBatch, LanceStringArray, LanceUInt64Array, VectorStore, VectorStoreError,
};

use crate::gateway::studio::types::{AstSearchHit, AutocompleteSuggestion};
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, trim_ranked_vec,
};

use crate::search_plane::local_symbol::schema::hit_json_column;

const MIN_RETAINED_LOCAL_SYMBOLS: usize = 64;
const RETAINED_LOCAL_SYMBOL_MULTIPLIER: usize = 4;
const MIN_RETAINED_AUTOCOMPLETE_SUGGESTIONS: usize = 16;
const RETAINED_AUTOCOMPLETE_MULTIPLIER: usize = 2;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LocalSymbolSearchError {
    #[error("local symbol index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

#[derive(Debug)]
pub(crate) struct LocalSymbolSearchExecution {
    pub(crate) candidates: Vec<LocalSymbolCandidate>,
    pub(crate) telemetry: StreamingRerankTelemetry,
    pub(crate) source: StreamingRerankSource,
}

#[derive(Debug)]
pub(crate) struct LocalSymbolAutocompleteExecution {
    pub(crate) suggestions: Vec<AutocompleteSuggestion>,
    pub(crate) telemetry: StreamingRerankTelemetry,
    pub(crate) source: StreamingRerankSource,
}

#[derive(Debug)]
pub(crate) struct LocalSymbolCandidate {
    pub(crate) score: f64,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) line_start: usize,
    pub(crate) hit_json: String,
}

pub(crate) fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(
        limit,
        RETAINED_LOCAL_SYMBOL_MULTIPLIER,
        MIN_RETAINED_LOCAL_SYMBOLS,
    )
}

pub(crate) fn suggestion_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(
        limit,
        RETAINED_AUTOCOMPLETE_MULTIPLIER,
        MIN_RETAINED_AUTOCOMPLETE_SUGGESTIONS,
    )
}

pub(crate) async fn execute_local_symbol_search(
    store: &VectorStore,
    table_names: &[String],
    query_lower: &str,
    options: xiuxian_vector::ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<LocalSymbolSearchExecution, LocalSymbolSearchError> {
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut candidates = Vec::with_capacity(window.target);
    store
        .scan_record_batches_streaming_across_tables(table_names, options, |_table_name, batch| {
            collect_candidates(&batch, query_lower, &mut candidates, window, &mut telemetry)
        })
        .await?;
    Ok(LocalSymbolSearchExecution {
        candidates,
        telemetry,
        source: StreamingRerankSource::Scan,
    })
}

pub(crate) async fn execute_local_symbol_autocomplete(
    store: &VectorStore,
    table_names: &[String],
    normalized_prefix: &str,
    options: xiuxian_vector::ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<LocalSymbolAutocompleteExecution, LocalSymbolSearchError> {
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut suggestions = Vec::with_capacity(window.target);
    let mut seen = HashSet::new();
    store
        .scan_record_batches_streaming_across_tables(table_names, options, |_table_name, batch| {
            collect_suggestions(
                &batch,
                normalized_prefix,
                &mut suggestions,
                &mut seen,
                window,
                &mut telemetry,
            )
        })
        .await?;
    Ok(LocalSymbolAutocompleteExecution {
        suggestions,
        telemetry,
        source: StreamingRerankSource::Scan,
    })
}

pub(crate) fn decode_local_symbol_hits(
    candidates: Vec<LocalSymbolCandidate>,
) -> Result<Vec<AstSearchHit>, LocalSymbolSearchError> {
    candidates
        .into_iter()
        .map(|candidate| {
            let mut hit: AstSearchHit = serde_json::from_str(candidate.hit_json.as_str())
                .map_err(|error| LocalSymbolSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

pub(crate) fn collect_candidates(
    batch: &LanceRecordBatch,
    query_lower: &str,
    candidates: &mut Vec<LocalSymbolCandidate>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), LocalSymbolSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let name = string_column(batch, "name")?;
    let name_folded = string_column(batch, "name_folded")?;
    let signature = string_column(batch, "signature")?;
    let owner_title = string_column(batch, "owner_title")?;
    let path = string_column(batch, "path")?;
    let line_start = u64_column(batch, "line_start")?;
    let hit_json = string_column(batch, hit_json_column())?;

    for row in 0..batch.num_rows() {
        let score = candidate_score(
            query_lower,
            name_folded.value(row),
            signature.value(row),
            if owner_title.is_null(row) {
                ""
            } else {
                owner_title.value(row)
            },
        );
        if score <= 0.0 {
            continue;
        }

        telemetry.observe_match();
        candidates.push(LocalSymbolCandidate {
            score,
            name: name.value(row).to_string(),
            path: path.value(row).to_string(),
            line_start: usize::try_from(line_start.value(row)).unwrap_or(usize::MAX),
            hit_json: hit_json.value(row).to_string(),
        });
        telemetry.observe_working_set(candidates.len());
        if candidates.len() > window.threshold {
            let before_len = candidates.len();
            trim_ranked_vec(candidates, window.target, compare_candidates);
            telemetry.observe_trim(before_len, candidates.len());
        }
    }

    Ok(())
}

pub(crate) fn collect_suggestions(
    batch: &LanceRecordBatch,
    normalized_prefix: &str,
    suggestions: &mut Vec<AutocompleteSuggestion>,
    seen: &mut HashSet<String>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), LocalSymbolSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let name = string_column(batch, "name")?;
    let name_folded = string_column(batch, "name_folded")?;
    let language = string_column(batch, "language")?;
    let node_kind = string_column(batch, "node_kind")?;

    for row in 0..batch.num_rows() {
        let text = name.value(row).trim();
        if text.is_empty()
            || !autocomplete_matches_prefix(name_folded.value(row), normalized_prefix)
        {
            continue;
        }

        let dedupe_key = name_folded.value(row).to_string();
        if !seen.insert(dedupe_key) {
            continue;
        }

        telemetry.observe_match();
        suggestions.push(AutocompleteSuggestion {
            text: text.to_string(),
            suggestion_type: autocomplete_suggestion_type(
                language.value(row),
                nullable_string_value(node_kind, row),
            )
            .to_string(),
        });
        telemetry.observe_working_set(suggestions.len());
        if suggestions.len() > window.threshold {
            let before_len = suggestions.len();
            trim_ranked_vec(suggestions, window.target, compare_suggestions);
            telemetry.observe_trim(before_len, suggestions.len());
        }
    }

    Ok(())
}

pub(crate) fn compare_candidates(
    left: &LocalSymbolCandidate,
    right: &LocalSymbolCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.line_start.cmp(&right.line_start))
}

pub(crate) fn compare_suggestions(
    left: &AutocompleteSuggestion,
    right: &AutocompleteSuggestion,
) -> std::cmp::Ordering {
    suggestion_rank(left)
        .cmp(&suggestion_rank(right))
        .then_with(|| left.text.cmp(&right.text))
}

pub(crate) fn candidate_score(
    query_lower: &str,
    name_folded: &str,
    signature: &str,
    owner_title: &str,
) -> f64 {
    if name_folded == query_lower {
        return 1.0;
    }
    if name_folded.starts_with(query_lower) {
        return 0.97;
    }
    if name_folded.contains(query_lower) {
        return 0.93;
    }

    let signature_folded = signature.to_ascii_lowercase();
    if signature_folded.contains(query_lower) {
        return 0.86;
    }
    let owner_folded = owner_title.to_ascii_lowercase();
    if !owner_folded.is_empty() && owner_folded.contains(query_lower) {
        return 0.81;
    }
    0.0
}

pub(crate) fn autocomplete_matches_prefix(normalized_text: &str, normalized_prefix: &str) -> bool {
    if normalized_text.starts_with(normalized_prefix) {
        return true;
    }

    normalized_text
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| !token.is_empty() && token.starts_with(normalized_prefix))
}

pub(crate) fn autocomplete_suggestion_type(
    language: &str,
    node_kind: Option<&str>,
) -> &'static str {
    if language != "markdown" {
        return "symbol";
    }

    match node_kind {
        Some("property" | "observation") => "metadata",
        _ => "heading",
    }
}

fn suggestion_rank(suggestion: &AutocompleteSuggestion) -> usize {
    match suggestion.suggestion_type.as_str() {
        "symbol" => 0,
        "heading" => 1,
        "metadata" => 2,
        _ => 3,
    }
}

fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, LocalSymbolSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| LocalSymbolSearchError::Decode(format!("missing string column `{name}`")))
}

fn u64_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceUInt64Array, LocalSymbolSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceUInt64Array>())
        .ok_or_else(|| LocalSymbolSearchError::Decode(format!("missing u64 column `{name}`")))
}

fn nullable_string_value(array: &LanceStringArray, row: usize) -> Option<&str> {
    (!array.is_null(row)).then(|| array.value(row))
}
