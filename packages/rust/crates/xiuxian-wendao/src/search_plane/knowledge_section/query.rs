use std::collections::HashMap;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, VectorStoreError,
};

use crate::gateway::studio::types::SearchHit;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::projected_columns;

#[derive(Debug, thiserror::Error)]
pub(crate) enum KnowledgeSectionSearchError {
    #[error("knowledge section index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

pub(crate) async fn search_knowledge_sections(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, KnowledgeSectionSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::KnowledgeSection);
    let Some(active_epoch) = status.active_epoch else {
        return Err(KnowledgeSectionSearchError::NotReady);
    };

    let query_text = query.trim();
    if query_text.is_empty() {
        return Ok(Vec::new());
    }

    let store = service
        .open_store(SearchCorpusKind::KnowledgeSection)
        .await?;
    let table_name = service.table_name(SearchCorpusKind::KnowledgeSection, active_epoch);
    let options = ColumnarScanOptions {
        projected_columns: projected_columns()
            .into_iter()
            .map(str::to_string)
            .collect(),
        batch_size: Some(256),
        limit: if should_use_fts(query_text) {
            Some(limit.saturating_mul(32).max(128))
        } else {
            None
        },
        ..ColumnarScanOptions::default()
    };
    let batches = if should_use_fts(query_text) {
        match store
            .search_fts_batches(table_name.as_str(), query_text, options.clone())
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
            Err(error) => return Err(KnowledgeSectionSearchError::Storage(error)),
        }
    } else {
        store
            .scan_record_batches(table_name.as_str(), options)
            .await?
    };

    let query_lower = query_text.to_ascii_lowercase();
    let mut best_by_path = HashMap::<String, KnowledgeCandidate>::new();
    for batch in &batches {
        collect_candidates(batch, query_text, query_lower.as_str(), &mut best_by_path)?;
    }

    let mut candidates = best_by_path.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.stem.cmp(&right.stem))
    });
    candidates.truncate(limit);

    candidates
        .into_iter()
        .map(|candidate| {
            let mut hit: SearchHit = serde_json::from_str(candidate.hit_json.as_str())
                .map_err(|error| KnowledgeSectionSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

#[derive(Debug)]
struct KnowledgeCandidate {
    path: String,
    stem: String,
    score: f64,
    hit_json: String,
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    query_text: &str,
    query_lower: &str,
    best_by_path: &mut HashMap<String, KnowledgeCandidate>,
) -> Result<(), KnowledgeSectionSearchError> {
    let path = string_column(batch, "path")?;
    let stem = string_column(batch, "stem")?;
    let title = string_column(batch, "title")?;
    let best_section = string_column(batch, "best_section")?;
    let search_text_folded = string_column(batch, "search_text_folded")?;
    let hit_json = string_column(batch, "hit_json")?;

    for row in 0..batch.num_rows() {
        let score = score_candidate(
            query_text,
            query_lower,
            stem.value(row),
            nullable_value(title, row),
            nullable_value(best_section, row),
            search_text_folded.value(row),
        );
        if score <= 0.0 {
            continue;
        }

        let candidate = KnowledgeCandidate {
            path: path.value(row).to_string(),
            stem: stem.value(row).to_string(),
            score,
            hit_json: hit_json.value(row).to_string(),
        };
        match best_by_path.get(candidate.path.as_str()) {
            Some(existing) if existing.score >= candidate.score => {}
            _ => {
                best_by_path.insert(candidate.path.clone(), candidate);
            }
        }
    }

    Ok(())
}

fn score_candidate(
    query_text: &str,
    query_lower: &str,
    stem: &str,
    title: Option<&str>,
    best_section: Option<&str>,
    search_text_folded: &str,
) -> f64 {
    if title.is_some_and(|value| value == query_text) {
        return 1.0;
    }
    if title.is_some_and(|value| value.to_ascii_lowercase().contains(query_lower)) {
        return 0.95;
    }
    if best_section.is_some_and(|value| value.to_ascii_lowercase().contains(query_lower)) {
        return 0.9;
    }
    if stem.to_ascii_lowercase().contains(query_lower) {
        return 0.88;
    }
    if search_text_folded.contains(query_lower) {
        return 0.82;
    }
    0.0
}

fn should_use_fts(query: &str) -> bool {
    query
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '_' || ch == '-')
}

fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, KnowledgeSectionSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| {
            KnowledgeSectionSearchError::Decode(format!("missing string column `{name}`"))
        })
}

fn nullable_value<'a>(array: &'a LanceStringArray, index: usize) -> Option<&'a str> {
    (!array.is_null(index)).then(|| array.value(index))
}
