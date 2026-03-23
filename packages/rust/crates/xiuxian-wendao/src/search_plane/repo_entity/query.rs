use std::collections::HashSet;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceFloat64Array, LanceRecordBatch, LanceStringArray,
    VectorStoreError,
};

use crate::analyzers::service::{
    example_match_score, module_match_score, normalized_rank_score, symbol_match_score,
};
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{hit_json_column, projected_columns};

const MODULE_BUCKETS: u8 = 3;
const SYMBOL_BUCKETS: u8 = 7;
const EXAMPLE_BUCKETS: u8 = 10;

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepoEntitySearchError {
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

pub(crate) async fn search_repo_entities(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    language_filters: &HashSet<String>,
    kind_filters: &HashSet<String>,
    limit: usize,
) -> Result<Vec<SearchHit>, RepoEntitySearchError> {
    let trimmed = query.trim();
    let query_lower = trimmed.to_ascii_lowercase();

    let store = service.open_store(SearchCorpusKind::RepoEntity).await?;
    let table_name = service.repo_entity_table_name(repo_id);
    if !store.table_path(table_name.as_str()).exists() {
        return Ok(Vec::new());
    }

    let mut columns = projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    columns.push(hit_json_column().to_string());
    let options = ColumnarScanOptions {
        where_filter: filter_expression(language_filters),
        projected_columns: columns,
        batch_size: Some(512),
        limit: if should_use_fts(trimmed) {
            Some(limit.saturating_mul(32).max(128))
        } else {
            None
        },
        ..ColumnarScanOptions::default()
    };

    let batches = if should_use_fts(trimmed) {
        match store
            .search_fts_batches(table_name.as_str(), trimmed, options.clone())
            .await
        {
            Ok(batches) if !batches.is_empty() => batches,
            Ok(_) => {
                store
                    .scan_record_batches(table_name.as_str(), options)
                    .await?
            }
            Err(VectorStoreError::LanceDB(_)) => {
                store
                    .scan_record_batches(table_name.as_str(), options)
                    .await?
            }
            Err(error) => return Err(RepoEntitySearchError::Storage(error)),
        }
    } else {
        store
            .scan_record_batches(table_name.as_str(), options)
            .await?
    };

    let mut candidates = Vec::new();
    for batch in &batches {
        collect_candidates(
            batch,
            &query_lower,
            language_filters,
            kind_filters,
            &mut candidates,
        )?;
    }

    candidates.sort_by(|left, right| {
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
    });
    candidates.truncate(limit);

    candidates
        .into_iter()
        .map(|candidate| {
            let mut hit: SearchHit = serde_json::from_str(candidate.hit_json.as_str())
                .map_err(|error| RepoEntitySearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

#[derive(Debug)]
struct RepoEntityCandidate {
    score: f64,
    entity_kind: String,
    name: String,
    path: String,
    hit_json: String,
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    query_lower: &str,
    language_filters: &HashSet<String>,
    kind_filters: &HashSet<String>,
    candidates: &mut Vec<RepoEntityCandidate>,
) -> Result<(), RepoEntitySearchError> {
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
    let hit_json = string_column(batch, hit_json_column())?;

    for row in 0..batch.num_rows() {
        let entity_kind_value = entity_kind.value(row);
        let language_value = language.value(row);
        let symbol_kind_value = symbol_kind.value(row);
        if !matches_language_filters(language_filters, language_value) {
            continue;
        }
        if !matches_kind_filters(kind_filters, entity_kind_value, symbol_kind_value) {
            continue;
        }

        let Some(normalized) = candidate_score(
            query_lower,
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

        let score = normalized.max(saliency_score.value(row)).clamp(0.0, 1.0);
        candidates.push(RepoEntityCandidate {
            score,
            entity_kind: entity_kind_value.to_string(),
            name: name.value(row).to_string(),
            path: path.value(row).to_string(),
            hit_json: hit_json.value(row).to_string(),
        });
        if candidates.len() > 512 {
            candidates.sort_by(|left, right| {
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
            });
            candidates.truncate(256);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
    let raw_score = match entity_kind {
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
    };
    raw_score
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
    query.chars().any(|value| value.is_alphanumeric()) && query.len() >= 2
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

fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, RepoEntitySearchError> {
    batch
        .column_by_name(name)
        .and_then(|array| array.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| RepoEntitySearchError::Decode(format!("missing string column `{name}`")))
}

fn float64_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceFloat64Array, RepoEntitySearchError> {
    batch
        .column_by_name(name)
        .and_then(|array| array.as_any().downcast_ref::<LanceFloat64Array>())
        .ok_or_else(|| RepoEntitySearchError::Decode(format!("missing f64 column `{name}`")))
}
