use std::collections::{HashMap, HashSet};

use xiuxian_vector::{ColumnarScanOptions, LanceArray, LanceRecordBatch, string_contains_mask};

use crate::search_plane::ranking::{RetainedWindow, trim_ranked_string_map};

use super::RepoContentChunkCandidate;
use super::RepoContentChunkSearchError;
use super::candidate_path_key;
use super::compare_candidates;
use super::helpers::{filter_expression, string_column, u64_column};
use super::should_use_fts;

const MIN_RETAINED_PATHS: usize = 128;
const RETAINED_PATH_MULTIPLIER: usize = 8;

pub(crate) fn build_repo_content_scan_options(
    language_filters: &HashSet<String>,
    trimmed: &str,
    limit: usize,
) -> ColumnarScanOptions {
    ColumnarScanOptions {
        where_filter: filter_expression(language_filters),
        projected_columns: super::helpers::projected_repo_content_columns(),
        batch_size: Some(512),
        limit: if should_use_fts(trimmed) {
            Some(limit.saturating_mul(32).max(128))
        } else {
            None
        },
        ..ColumnarScanOptions::default()
    }
}

pub(crate) fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(limit, RETAINED_PATH_MULTIPLIER, MIN_RETAINED_PATHS)
}

pub(crate) fn collect_candidates(
    batch: &LanceRecordBatch,
    raw_needle: &str,
    needle: &str,
    best_by_path: &mut HashMap<String, RepoContentChunkCandidate>,
    window: RetainedWindow,
    telemetry: &mut crate::search_plane::ranking::StreamingRerankTelemetry,
) -> Result<(), RepoContentChunkSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let path = string_column(batch, "path")?;
    let language = string_column(batch, "language")?;
    let line_number = u64_column(batch, "line_number")?;
    let line_text = string_column(batch, "line_text")?;
    let line_text_folded = string_column(batch, "line_text_folded")?;
    let contains_mask = string_contains_mask(line_text_folded, needle)?;

    for row in 0..contains_mask.len() {
        if contains_mask.is_null(row) || !contains_mask.value(row) {
            continue;
        }
        let exact_match = line_text.value(row).contains(raw_needle);
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
