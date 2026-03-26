use xiuxian_vector::{LanceArray, LanceRecordBatch, LanceStringArray};

use crate::search_plane::knowledge_section::query::candidates::KnowledgeCandidate;
use crate::search_plane::knowledge_section::query::errors::KnowledgeSectionSearchError;

pub(crate) fn compare_candidates(
    left: &KnowledgeCandidate,
    right: &KnowledgeCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.stem.cmp(&right.stem))
}

pub(crate) fn candidate_path_key(candidate: &KnowledgeCandidate) -> String {
    candidate.path.clone()
}

pub(crate) fn score_candidate(
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

pub(crate) fn should_use_fts(query: &str) -> bool {
    query
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '_' || ch == '-')
}

pub(crate) fn string_column<'a>(
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

pub(crate) fn nullable_value(array: &LanceStringArray, index: usize) -> Option<&str> {
    (!array.is_null(index)).then(|| array.value(index))
}
