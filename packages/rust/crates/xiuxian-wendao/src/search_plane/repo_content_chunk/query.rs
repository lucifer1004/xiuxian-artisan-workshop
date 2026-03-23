use std::collections::{HashMap, HashSet};
use std::path::Path;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, LanceUInt64Array,
    VectorStoreError,
};

use crate::gateway::studio::types::{SearchHit, StudioNavigationTarget};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{language_column, projected_columns};

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
    let table_name = service.repo_content_chunk_table_name(repo_id);
    if !store.table_path(table_name.as_str()).exists() {
        return Ok(Vec::new());
    }

    let options = ColumnarScanOptions {
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
            Err(error) => return Err(RepoContentChunkSearchError::Storage(error)),
        }
    } else {
        store
            .scan_record_batches(table_name.as_str(), options)
            .await?
    };

    let needle = trimmed.to_ascii_lowercase();
    let mut best_by_path = HashMap::<String, RepoContentChunkCandidate>::new();
    for batch in &batches {
        collect_candidates(batch, trimmed, needle.as_str(), &mut best_by_path)?;
    }

    let mut hits = best_by_path.into_values().collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_number.cmp(&right.line_number))
    });
    hits.truncate(limit);
    Ok(hits
        .into_iter()
        .map(|candidate| candidate.into_search_hit(repo_id))
        .collect())
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
) -> Result<(), RepoContentChunkSearchError> {
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
            }
        }
    }

    Ok(())
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
