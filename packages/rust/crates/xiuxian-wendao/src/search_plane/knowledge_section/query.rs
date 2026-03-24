use std::collections::HashMap;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, VectorStore,
    VectorStoreError,
};

use crate::gateway::studio::types::SearchHit;
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, sort_by_rank,
    trim_ranked_string_map,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::projected_columns;

const MIN_RETAINED_PATHS: usize = 128;
const RETAINED_PATH_MULTIPLIER: usize = 8;

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
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, active_epoch);
    let options = build_knowledge_scan_options(query_text, limit);
    let query_lower = query_text.to_ascii_lowercase();
    let execution = execute_knowledge_search(
        &store,
        table_name.as_str(),
        query_text,
        query_lower.as_str(),
        options,
        retained_window(limit),
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits = decode_knowledge_hits(candidates)?;
    service.record_query_telemetry(
        SearchCorpusKind::KnowledgeSection,
        execution
            .telemetry
            .finish(execution.source, None, hits.len()),
    );
    Ok(hits)
}

fn build_knowledge_scan_options(query_text: &str, limit: usize) -> ColumnarScanOptions {
    ColumnarScanOptions {
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
    }
}

#[derive(Debug, Clone)]
struct KnowledgeCandidate {
    path: String,
    stem: String,
    score: f64,
    hit_json: String,
}

struct KnowledgeSearchExecution {
    candidates: Vec<KnowledgeCandidate>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

async fn execute_knowledge_search(
    store: &VectorStore,
    table_name: &str,
    query_text: &str,
    query_lower: &str,
    options: ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<KnowledgeSearchExecution, KnowledgeSectionSearchError> {
    let fts_eligible = should_use_fts(query_text);
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut saw_fts_batch = false;
    let mut fell_back_to_scan = false;
    let mut best_by_path = HashMap::<String, KnowledgeCandidate>::with_capacity(window.target);

    if fts_eligible {
        match store
            .search_fts_batches_streaming(table_name, query_text, options.clone(), |batch| {
                saw_fts_batch = true;
                collect_candidates(
                    &batch,
                    query_text,
                    query_lower,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await
        {
            Ok(()) if saw_fts_batch => {}
            Ok(()) | Err(KnowledgeSectionSearchError::Storage(VectorStoreError::LanceDB(_))) => {
                fell_back_to_scan = true;
                best_by_path.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(
                            &batch,
                            query_text,
                            query_lower,
                            &mut best_by_path,
                            window,
                            &mut telemetry,
                        )
                    })
                    .await?;
            }
            Err(error) => return Err(error),
        }
    } else {
        store
            .scan_record_batches_streaming(table_name, options, |batch| {
                collect_candidates(
                    &batch,
                    query_text,
                    query_lower,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await?;
    }

    Ok(KnowledgeSearchExecution {
        candidates: best_by_path.into_values().collect(),
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
    query_text: &str,
    query_lower: &str,
    best_by_path: &mut HashMap<String, KnowledgeCandidate>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), KnowledgeSectionSearchError> {
    telemetry.observe_batch(batch.num_rows());
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

        telemetry.observe_match();
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
                telemetry.observe_working_set(best_by_path.len());
                if best_by_path.len() > window.threshold {
                    let before_len = best_by_path.len();
                    trim_ranked_string_map(
                        best_by_path,
                        window.target,
                        compare_candidates,
                        candidate_path_key,
                    );
                    telemetry.observe_trim(before_len, best_by_path.len());
                }
            }
        }
    }

    Ok(())
}

fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(limit, RETAINED_PATH_MULTIPLIER, MIN_RETAINED_PATHS)
}

fn decode_knowledge_hits(
    candidates: Vec<KnowledgeCandidate>,
) -> Result<Vec<SearchHit>, KnowledgeSectionSearchError> {
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

fn compare_candidates(left: &KnowledgeCandidate, right: &KnowledgeCandidate) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.stem.cmp(&right.stem))
}

fn candidate_path_key(candidate: &KnowledgeCandidate) -> String {
    candidate.path.clone()
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

fn nullable_value(array: &LanceStringArray, index: usize) -> Option<&str> {
    (!array.is_null(index)).then(|| array.value(index))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{KnowledgeCandidate, candidate_path_key, compare_candidates, retained_window};
    use crate::search_plane::ranking::trim_ranked_string_map;

    #[test]
    fn trim_best_by_path_keeps_highest_ranked_hits() {
        let mut best_by_path = HashMap::from([
            (
                "notes/zeta.md".to_string(),
                KnowledgeCandidate {
                    path: "notes/zeta.md".to_string(),
                    stem: "zeta".to_string(),
                    score: 0.82,
                    hit_json: "{}".to_string(),
                },
            ),
            (
                "notes/beta.md".to_string(),
                KnowledgeCandidate {
                    path: "notes/beta.md".to_string(),
                    stem: "beta".to_string(),
                    score: 0.95,
                    hit_json: "{}".to_string(),
                },
            ),
            (
                "notes/alpha.md".to_string(),
                KnowledgeCandidate {
                    path: "notes/alpha.md".to_string(),
                    stem: "alpha".to_string(),
                    score: 0.95,
                    hit_json: "{}".to_string(),
                },
            ),
        ]);

        trim_ranked_string_map(&mut best_by_path, 2, compare_candidates, candidate_path_key);

        let mut retained = best_by_path.into_values().collect::<Vec<_>>();
        retained.sort_by(compare_candidates);
        assert_eq!(retained.len(), 2);
        assert_eq!(retained[0].path, "notes/alpha.md");
        assert_eq!(retained[1].path, "notes/beta.md");
    }

    #[test]
    fn retained_window_scales_with_limit() {
        assert_eq!(retained_window(0).target, 128);
        assert_eq!(retained_window(4).target, 128);
        assert_eq!(retained_window(64).target, 512);
    }
}
