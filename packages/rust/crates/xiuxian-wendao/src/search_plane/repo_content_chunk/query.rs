use std::collections::{HashMap, HashSet};
use std::path::Path;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, LanceUInt64Array,
    VectorStore, VectorStoreError,
};

use crate::gateway::studio::types::{SearchHit, StudioNavigationTarget};
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, sort_by_rank,
    trim_ranked_string_map,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{language_column, projected_columns};

const MIN_RETAINED_PATHS: usize = 128;
const RETAINED_PATH_MULTIPLIER: usize = 8;

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepoContentChunkSearchError {
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

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

    let store = service
        .open_store(SearchCorpusKind::RepoContentChunk)
        .await?;
    let table_name = SearchPlaneService::repo_content_chunk_table_name(repo_id);
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

#[derive(Debug, Clone)]
struct RepoContentChunkCandidate {
    path: String,
    language: Option<String>,
    line_number: usize,
    line_text: String,
    score: f64,
    exact_match: bool,
}

struct RepoContentChunkSearchExecution {
    candidates: Vec<RepoContentChunkCandidate>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

async fn execute_repo_content_search(
    store: &VectorStore,
    table_name: &str,
    raw_needle: &str,
    needle: &str,
    options: ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<RepoContentChunkSearchExecution, RepoContentChunkSearchError> {
    let fts_eligible = should_use_fts(raw_needle);
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut saw_fts_batch = false;
    let mut fell_back_to_scan = false;
    let mut best_by_path =
        HashMap::<String, RepoContentChunkCandidate>::with_capacity(window.target);

    if fts_eligible {
        match store
            .search_fts_batches_streaming(table_name, raw_needle, options.clone(), |batch| {
                saw_fts_batch = true;
                collect_candidates(
                    &batch,
                    raw_needle,
                    needle,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await
        {
            Ok(()) if saw_fts_batch => {}
            Ok(()) | Err(RepoContentChunkSearchError::Storage(VectorStoreError::LanceDB(_))) => {
                fell_back_to_scan = true;
                best_by_path.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(
                            &batch,
                            raw_needle,
                            needle,
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
                    raw_needle,
                    needle,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await?;
    }

    Ok(RepoContentChunkSearchExecution {
        candidates: best_by_path.into_values().collect(),
        telemetry,
        source: match (fts_eligible, fell_back_to_scan) {
            (true, true) => StreamingRerankSource::FtsFallbackScan,
            (true, false) => StreamingRerankSource::Fts,
            (false, _) => StreamingRerankSource::Scan,
        },
    })
}

impl RepoContentChunkCandidate {
    fn into_search_hit(self, repo_id: &str) -> SearchHit {
        let mut tags = vec![
            repo_id.to_string(),
            "code".to_string(),
            "file".to_string(),
            "kind:file".to_string(),
        ];
        if let Some(language) = self
            .language
            .clone()
            .or_else(|| infer_code_language(self.path.as_str()))
        {
            tags.push(language.clone());
            tags.push(format!("lang:{language}"));
        }
        let stem = Path::new(self.path.as_str())
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(self.path.as_str())
            .to_string();

        SearchHit {
            stem,
            title: Some(self.path.clone()),
            path: self.path.clone(),
            doc_type: Some("file".to_string()),
            tags,
            score: self.score,
            best_section: Some(format!(
                "{}: {}",
                self.line_number,
                truncate_content_search_snippet(self.line_text.as_str(), 140)
            )),
            match_reason: Some("repo_content_search".to_string()),
            hierarchical_uri: None,
            hierarchy: Some(self.path.split('/').map(str::to_string).collect::<Vec<_>>()),
            implicit_backlinks: None,
            implicit_backlink_items: None,
            audit_status: None,
            verification_state: None,
            saliency_score: None,
            navigation_target: Some(StudioNavigationTarget {
                path: format!("{repo_id}/{}", self.path),
                category: "repo_code".to_string(),
                project_name: Some(repo_id.to_string()),
                root_label: Some(repo_id.to_string()),
                line: Some(self.line_number),
                line_end: Some(self.line_number),
                column: None,
            }),
        }
    }
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    raw_needle: &str,
    needle: &str,
    best_by_path: &mut HashMap<String, RepoContentChunkCandidate>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), RepoContentChunkSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let path = string_column(batch, "path")?;
    let language = string_column(batch, "language")?;
    let line_number = u64_column(batch, "line_number")?;
    let line_text = string_column(batch, "line_text")?;
    let line_text_folded = string_column(batch, "line_text_folded")?;

    for row in 0..batch.num_rows() {
        let exact_match = line_text.value(row).contains(raw_needle);
        if !exact_match && !line_text_folded.value(row).contains(needle) {
            continue;
        }

        telemetry.observe_match();
        let candidate = RepoContentChunkCandidate {
            path: path.value(row).to_string(),
            language: (!language.value(row).trim().is_empty())
                .then(|| language.value(row).to_string()),
            line_number: usize::try_from(line_number.value(row)).unwrap_or(usize::MAX),
            line_text: line_text.value(row).to_string(),
            score: if exact_match { 0.73 } else { 0.72 },
            exact_match,
        };

        match best_by_path.get(candidate.path.as_str()) {
            Some(existing) if existing.exact_match && !candidate.exact_match => {}
            Some(existing)
                if existing.exact_match == candidate.exact_match
                    && existing.line_number <= candidate.line_number => {}
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

fn build_repo_content_scan_options(
    language_filters: &HashSet<String>,
    trimmed: &str,
    limit: usize,
) -> ColumnarScanOptions {
    ColumnarScanOptions {
        where_filter: filter_expression(language_filters),
        projected_columns: projected_columns()
            .into_iter()
            .map(str::to_string)
            .collect(),
        batch_size: Some(512),
        limit: if should_use_fts(trimmed) {
            Some(limit.saturating_mul(32).max(128))
        } else {
            None
        },
        ..ColumnarScanOptions::default()
    }
}

fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(limit, RETAINED_PATH_MULTIPLIER, MIN_RETAINED_PATHS)
}

fn compare_candidates(
    left: &RepoContentChunkCandidate,
    right: &RepoContentChunkCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.line_number.cmp(&right.line_number))
}

fn candidate_path_key(candidate: &RepoContentChunkCandidate) -> String {
    candidate.path.clone()
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
            .map(|value| {
                format!(
                    "{column} = '{}'",
                    escape_literal(value.as_str()),
                    column = language_column()
                )
            })
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

fn escape_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn should_use_fts(query: &str) -> bool {
    query
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '_' || ch == '-')
}

fn infer_code_language(path: &str) -> Option<String> {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("jl") || ext.eq_ignore_ascii_case("julia") => {
            Some("julia".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("mo") || ext.eq_ignore_ascii_case("modelica") => {
            Some("modelica".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("rs") => Some("rust".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("py") => Some("python".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("ts") || ext.eq_ignore_ascii_case("tsx") => {
            Some("typescript".to_string())
        }
        _ => None,
    }
}

fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, RepoContentChunkSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| {
            RepoContentChunkSearchError::Decode(format!("missing string column `{name}`"))
        })
}

fn u64_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceUInt64Array, RepoContentChunkSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceUInt64Array>())
        .ok_or_else(|| RepoContentChunkSearchError::Decode(format!("missing u64 column `{name}`")))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        RepoContentChunkCandidate, candidate_path_key, compare_candidates, retained_window,
    };
    use crate::search_plane::ranking::trim_ranked_string_map;

    #[test]
    fn trim_best_by_path_keeps_top_ranked_paths() {
        let mut best_by_path = HashMap::from([
            (
                "src/zeta.jl".to_string(),
                RepoContentChunkCandidate {
                    path: "src/zeta.jl".to_string(),
                    language: Some("julia".to_string()),
                    line_number: 30,
                    line_text: "zeta".to_string(),
                    score: 0.72,
                    exact_match: false,
                },
            ),
            (
                "src/beta.jl".to_string(),
                RepoContentChunkCandidate {
                    path: "src/beta.jl".to_string(),
                    language: Some("julia".to_string()),
                    line_number: 20,
                    line_text: "beta".to_string(),
                    score: 0.73,
                    exact_match: true,
                },
            ),
            (
                "src/alpha.jl".to_string(),
                RepoContentChunkCandidate {
                    path: "src/alpha.jl".to_string(),
                    language: Some("julia".to_string()),
                    line_number: 10,
                    line_text: "alpha".to_string(),
                    score: 0.73,
                    exact_match: true,
                },
            ),
        ]);

        trim_ranked_string_map(&mut best_by_path, 2, compare_candidates, candidate_path_key);

        let mut retained = best_by_path.into_values().collect::<Vec<_>>();
        retained.sort_by(compare_candidates);
        assert_eq!(retained.len(), 2);
        assert_eq!(retained[0].path, "src/alpha.jl");
        assert_eq!(retained[1].path, "src/beta.jl");
    }

    #[test]
    fn retained_window_scales_with_limit() {
        assert_eq!(retained_window(0).target, 128);
        assert_eq!(retained_window(4).target, 128);
        assert_eq!(retained_window(64).target, 512);
    }
}
