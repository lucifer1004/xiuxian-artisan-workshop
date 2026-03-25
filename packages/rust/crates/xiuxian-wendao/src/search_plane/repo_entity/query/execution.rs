use std::collections::HashSet;

use xiuxian_vector::{ColumnarScanOptions, LanceRecordBatch, VectorStore, VectorStoreError};

use crate::analyzers::service::{
    example_match_score, module_match_score, normalized_rank_score, symbol_match_score,
};
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, trim_ranked_vec,
};
use crate::search_plane::repo_entity::query::hydrate::{float64_column, string_column};
use crate::search_plane::repo_entity::query::types::{
    EXAMPLE_BUCKETS, MIN_RECALL_CANDIDATES, MODULE_BUCKETS, RECALL_TRIM_MULTIPLIER,
    RepoEntityCandidate, RepoEntityQuery, RepoEntitySearchError, RepoEntitySearchExecution,
    SYMBOL_BUCKETS,
};

pub(crate) async fn execute_repo_entity_search(
    store: &VectorStore,
    table_name: &str,
    options: ColumnarScanOptions,
    query: &RepoEntityQuery<'_>,
) -> Result<RepoEntitySearchExecution, RepoEntitySearchError> {
    let fts_eligible = should_use_fts(query.query_text);
    let mut telemetry =
        StreamingRerankTelemetry::new(query.window, options.batch_size, options.limit);
    let mut candidates = Vec::with_capacity(query.window.target);
    let mut saw_fts_batch = false;
    let mut fell_back_to_scan = false;

    if fts_eligible {
        match store
            .search_fts_batches_streaming(table_name, query.query_text, options.clone(), |batch| {
                saw_fts_batch = true;
                collect_candidates(&batch, query, &mut candidates, &mut telemetry)
            })
            .await
        {
            Ok(()) if saw_fts_batch => {}
            Ok(()) => {
                fell_back_to_scan = true;
                candidates.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(&batch, query, &mut candidates, &mut telemetry)
                    })
                    .await?;
            }
            Err(error)
                if matches!(
                    error,
                    RepoEntitySearchError::Storage(VectorStoreError::LanceDB(_))
                ) || fts_batch_missing_projected_columns(&error) =>
            {
                fell_back_to_scan = true;
                candidates.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(&batch, query, &mut candidates, &mut telemetry)
                    })
                    .await?;
            }
            Err(error) => return Err(error),
        }
    } else {
        store
            .scan_record_batches_streaming(table_name, options, |batch| {
                collect_candidates(&batch, query, &mut candidates, &mut telemetry)
            })
            .await?;
    }

    Ok(RepoEntitySearchExecution {
        candidates,
        telemetry,
        source: match (fts_eligible, fell_back_to_scan) {
            (true, true) => StreamingRerankSource::FtsFallbackScan,
            (true, false) => StreamingRerankSource::Fts,
            (false, _) => StreamingRerankSource::Scan,
        },
    })
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    query: &RepoEntityQuery<'_>,
    candidates: &mut Vec<RepoEntityCandidate>,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), RepoEntitySearchError> {
    telemetry.observe_batch(batch.num_rows());
    let id = string_column(batch, "id")?;
    let entity_kind = string_column(batch, "entity_kind")?;
    let name = string_column(batch, "name")?;
    let name_folded = string_column(batch, "name_folded")?;
    let qualified_name_folded = string_column(batch, "qualified_name_folded")?;
    let path = string_column(batch, "path")?;
    let path_folded = string_column(batch, "path_folded")?;
    let language = string_column(batch, "language")?;
    let symbol_kind = string_column(batch, "symbol_kind")?;
    let signature_folded = string_column(batch, "signature_folded")?;
    let summary_folded = string_column(batch, "summary_folded")?;
    let related_symbols_folded = string_column(batch, "related_symbols_folded")?;
    let related_modules_folded = string_column(batch, "related_modules_folded")?;
    let saliency_score = float64_column(batch, "saliency_score")?;

    for row in 0..batch.num_rows() {
        let entity_kind_value = entity_kind.value(row);
        let language_value = language.value(row);
        let symbol_kind_value = symbol_kind.value(row);
        if !matches_language_filters(query.language_filters, language_value) {
            continue;
        }
        if !matches_kind_filters(query.kind_filters, entity_kind_value, symbol_kind_value) {
            continue;
        }

        let Some(normalized) = candidate_score(
            query.query_lower,
            entity_kind_value,
            name_folded.value(row),
            qualified_name_folded.value(row),
            path_folded.value(row),
            signature_folded.value(row),
            summary_folded.value(row),
            related_symbols_folded.value(row),
            related_modules_folded.value(row),
        ) else {
            continue;
        };

        telemetry.observe_match();
        let score = normalized.max(saliency_score.value(row)).clamp(0.0, 1.0);
        candidates.push(RepoEntityCandidate {
            id: id.value(row).to_string(),
            score,
            entity_kind: entity_kind_value.to_string(),
            name: name.value(row).to_string(),
            path: path.value(row).to_string(),
        });
        telemetry.observe_working_set(candidates.len());
        if candidates.len() > query.window.threshold {
            let before_len = candidates.len();
            trim_ranked_vec(candidates, query.window.target, compare_candidates);
            telemetry.observe_trim(before_len, candidates.len());
        }
    }

    Ok(())
}

pub(crate) fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(limit, RECALL_TRIM_MULTIPLIER, MIN_RECALL_CANDIDATES)
}

pub(crate) fn fixed_kind_filters(kind: &str) -> HashSet<String> {
    HashSet::from([kind.to_string()])
}

pub(crate) fn compare_candidates(
    left: &RepoEntityCandidate,
    right: &RepoEntityCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            candidate_kind_priority(right.entity_kind.as_str())
                .cmp(&candidate_kind_priority(left.entity_kind.as_str()))
        })
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.name.cmp(&right.name))
}

fn candidate_score(
    query_lower: &str,
    entity_kind: &str,
    name_folded: &str,
    qualified_name_folded: &str,
    path_folded: &str,
    signature_folded: &str,
    summary_folded: &str,
    related_symbols_folded: &str,
    related_modules_folded: &str,
) -> Option<f64> {
    match entity_kind {
        "module" => module_match_score(query_lower, qualified_name_folded, path_folded)
            .map(|score| normalized_rank_score(score, MODULE_BUCKETS)),
        "symbol" => symbol_match_score(
            query_lower,
            name_folded,
            qualified_name_folded,
            path_folded,
            signature_folded,
        )
        .map(|score| normalized_rank_score(score, SYMBOL_BUCKETS)),
        "example" => {
            let related_symbols = split_folded_values(related_symbols_folded);
            let related_modules = split_folded_values(related_modules_folded);
            example_match_score(
                query_lower,
                name_folded,
                path_folded,
                summary_folded,
                related_symbols.as_slice(),
                related_modules.as_slice(),
            )
            .map(|score| normalized_rank_score(score, EXAMPLE_BUCKETS))
        }
        _ => None,
    }
}

fn split_folded_values(value: &str) -> Vec<String> {
    value
        .split('\n')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(str::to_string)
        .collect()
}

fn matches_language_filters(filters: &HashSet<String>, language: &str) -> bool {
    filters.is_empty() || filters.contains(language)
}

fn matches_kind_filters(
    kind_filters: &HashSet<String>,
    entity_kind: &str,
    symbol_kind: &str,
) -> bool {
    if kind_filters.is_empty() {
        return true;
    }

    match entity_kind {
        "symbol" => {
            kind_filters.contains("symbol")
                || (!symbol_kind.is_empty() && kind_filters.contains(symbol_kind))
        }
        "module" => kind_filters.contains("module"),
        "example" => kind_filters.contains("example"),
        _ => false,
    }
}

fn candidate_kind_priority(entity_kind: &str) -> u8 {
    match entity_kind {
        "symbol" => 3,
        "module" => 2,
        "example" => 1,
        _ => 0,
    }
}

fn should_use_fts(query: &str) -> bool {
    query.chars().any(char::is_alphanumeric) && query.len() >= 2
}

fn fts_batch_missing_projected_columns(error: &RepoEntitySearchError) -> bool {
    match error {
        RepoEntitySearchError::Decode(message) => message.starts_with("missing "),
        RepoEntitySearchError::Storage(_) => false,
    }
}
