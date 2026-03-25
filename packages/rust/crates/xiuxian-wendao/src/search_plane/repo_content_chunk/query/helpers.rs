use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;

use xiuxian_vector::{LanceRecordBatch, LanceStringArray, LanceUInt64Array};

use crate::search_plane::repo_content_chunk::schema::{language_column, projected_columns};

use super::RepoContentChunkCandidate;
use super::RepoContentChunkSearchError;

pub(crate) fn compare_candidates(
    left: &RepoContentChunkCandidate,
    right: &RepoContentChunkCandidate,
) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.line_number.cmp(&right.line_number))
}

pub(crate) fn candidate_path_key(candidate: &RepoContentChunkCandidate) -> String {
    candidate.path.clone()
}

pub(crate) fn filter_expression(language_filters: &HashSet<String>) -> Option<String> {
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

pub(crate) fn should_use_fts(query: &str) -> bool {
    query.chars().any(|ch| ch.is_ascii_alphanumeric())
        && query.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || ch.is_ascii_whitespace()
                || matches!(ch, '_' | '-' | '.' | '/' | ':' | '(' | ')' | '@')
        })
}

pub(crate) fn infer_code_language(path: &str) -> Option<String> {
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

pub(crate) fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(crate) fn string_column<'a>(
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

pub(crate) fn u64_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceUInt64Array, RepoContentChunkSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceUInt64Array>())
        .ok_or_else(|| RepoContentChunkSearchError::Decode(format!("missing u64 column `{name}`")))
}

pub(crate) fn projected_repo_content_columns() -> Vec<String> {
    projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect()
}
