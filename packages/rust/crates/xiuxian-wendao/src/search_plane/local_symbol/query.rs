use std::collections::HashSet;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, LanceUInt64Array,
    VectorStoreError,
};

use crate::gateway::studio::types::{AstSearchHit, AutocompleteSuggestion};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{hit_json_column, projected_columns, suggestion_columns};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LocalSymbolSearchError {
    #[error("local symbol index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

pub(crate) async fn search_local_symbols(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
) -> Result<Vec<AstSearchHit>, LocalSymbolSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    let Some(active_epoch) = status.active_epoch else {
        return Err(LocalSymbolSearchError::NotReady);
    };

    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let table_name = service.table_name(SearchCorpusKind::LocalSymbol, active_epoch);
    let mut columns = projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    columns.push(hit_json_column().to_string());
    let batches = store
        .scan_record_batches(
            table_name.as_str(),
            ColumnarScanOptions {
                projected_columns: columns,
                batch_size: Some(256),
                limit: Some(limit.saturating_mul(32).max(128)),
                ..ColumnarScanOptions::default()
            },
        )
        .await?;

    let query_lower = query.trim().to_ascii_lowercase();
    let mut candidates = Vec::new();
    for batch in &batches {
        collect_candidates(batch, query_lower.as_str(), limit, &mut candidates)?;
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    candidates.truncate(limit);

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

pub(crate) async fn autocomplete_local_symbols(
    service: &SearchPlaneService,
    prefix: &str,
    limit: usize,
) -> Result<Vec<AutocompleteSuggestion>, LocalSymbolSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    let Some(active_epoch) = status.active_epoch else {
        return Err(LocalSymbolSearchError::NotReady);
    };

    let normalized_prefix = prefix.trim().to_ascii_lowercase();
    if normalized_prefix.is_empty() {
        return Ok(Vec::new());
    }

    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let table_name = service.table_name(SearchCorpusKind::LocalSymbol, active_epoch);
    let batches = store
        .scan_record_batches(
            table_name.as_str(),
            ColumnarScanOptions {
                projected_columns: suggestion_columns()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                batch_size: Some(256),
                limit: Some(limit.saturating_mul(64).max(256)),
                ..ColumnarScanOptions::default()
            },
        )
        .await?;

    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();
    for batch in &batches {
        collect_suggestions(
            batch,
            normalized_prefix.as_str(),
            limit,
            &mut suggestions,
            &mut seen,
        )?;
    }

    suggestions.sort_by(|left, right| {
        suggestion_rank(left)
            .cmp(&suggestion_rank(right))
            .then_with(|| left.text.cmp(&right.text))
    });
    suggestions.truncate(limit);
    Ok(suggestions)
}

#[derive(Debug)]
struct LocalSymbolCandidate {
    score: f64,
    name: String,
    path: String,
    line_start: usize,
    hit_json: String,
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    query_lower: &str,
    limit: usize,
    candidates: &mut Vec<LocalSymbolCandidate>,
) -> Result<(), LocalSymbolSearchError> {
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
            owner_title
                .is_null(row)
                .then_some("")
                .unwrap_or_else(|| owner_title.value(row)),
        );
        if score <= 0.0 {
            continue;
        }

        candidates.push(LocalSymbolCandidate {
            score,
            name: name.value(row).to_string(),
            path: path.value(row).to_string(),
            line_start: usize::try_from(line_start.value(row)).unwrap_or(usize::MAX),
            hit_json: hit_json.value(row).to_string(),
        });
        if candidates.len() > limit.saturating_mul(4).max(64) {
            candidates.sort_by(|left, right| {
                right
                    .score
                    .partial_cmp(&left.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.name.cmp(&right.name))
                    .then_with(|| left.path.cmp(&right.path))
                    .then_with(|| left.line_start.cmp(&right.line_start))
            });
            candidates.truncate(limit.saturating_mul(2).max(32));
        }
    }

    Ok(())
}

fn collect_suggestions(
    batch: &LanceRecordBatch,
    normalized_prefix: &str,
    limit: usize,
    suggestions: &mut Vec<AutocompleteSuggestion>,
    seen: &mut HashSet<String>,
) -> Result<(), LocalSymbolSearchError> {
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

        suggestions.push(AutocompleteSuggestion {
            text: text.to_string(),
            suggestion_type: autocomplete_suggestion_type(
                language.value(row),
                nullable_string_value(node_kind, row),
            )
            .to_string(),
        });
        if suggestions.len() > limit.saturating_mul(4).max(32) {
            suggestions.sort_by(|left, right| {
                suggestion_rank(left)
                    .cmp(&suggestion_rank(right))
                    .then_with(|| left.text.cmp(&right.text))
            });
            suggestions.truncate(limit.saturating_mul(2).max(16));
        }
    }

    Ok(())
}

fn candidate_score(
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

fn nullable_string_value<'a>(array: &'a LanceStringArray, row: usize) -> Option<&'a str> {
    (!array.is_null(row)).then(|| array.value(row))
}

fn autocomplete_matches_prefix(normalized_text: &str, normalized_prefix: &str) -> bool {
    if normalized_text.starts_with(normalized_prefix) {
        return true;
    }

    normalized_text
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| !token.is_empty() && token.starts_with(normalized_prefix))
}

fn autocomplete_suggestion_type(language: &str, node_kind: Option<&str>) -> &'static str {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::gateway::studio::types::{AstSearchHit, StudioNavigationTarget};
    use crate::search_plane::{
        BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
        SearchPlaneService,
    };

    use super::*;
    use crate::search_plane::local_symbol::schema::{local_symbol_batches, local_symbol_schema};

    fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
        SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:local_symbol"),
            SearchMaintenancePolicy::default(),
        )
    }

    fn sample_hit(name: &str, path: &str, line_start: usize) -> AstSearchHit {
        AstSearchHit {
            name: name.to_string(),
            signature: format!("fn {name}()"),
            path: path.to_string(),
            language: "rust".to_string(),
            crate_name: "kernel".to_string(),
            project_name: None,
            root_label: None,
            node_kind: None,
            owner_title: None,
            navigation_target: StudioNavigationTarget {
                path: path.to_string(),
                category: "symbol".to_string(),
                project_name: None,
                root_label: None,
                line: Some(line_start),
                line_end: Some(line_start),
                column: Some(1),
            },
            line_start,
            line_end: line_start,
            score: 0.0,
        }
    }

    fn sample_markdown_hit(
        name: &str,
        node_kind: Option<&str>,
        owner_title: Option<&str>,
    ) -> AstSearchHit {
        AstSearchHit {
            name: name.to_string(),
            signature: format!("## {name}"),
            path: "docs/alpha.md".to_string(),
            language: "markdown".to_string(),
            crate_name: "docs".to_string(),
            project_name: None,
            root_label: None,
            node_kind: node_kind.map(ToOwned::to_owned),
            owner_title: owner_title.map(ToOwned::to_owned),
            navigation_target: StudioNavigationTarget {
                path: "docs/alpha.md".to_string(),
                category: "symbol".to_string(),
                project_name: None,
                root_label: None,
                line: Some(1),
                line_end: Some(1),
                column: Some(1),
            },
            line_start: 1,
            line_end: 1,
            score: 0.0,
        }
    }

    #[tokio::test]
    async fn local_symbol_query_reads_hits_from_published_epoch() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = fixture_service(&temp_dir);
        let lease = match service.coordinator().begin_build(
            SearchCorpusKind::LocalSymbol,
            "fp-1",
            SearchCorpusKind::LocalSymbol.schema_version(),
        ) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin decision: {other:?}"),
        };
        let hits = vec![
            sample_hit("AlphaSymbol", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ];
        let store = service
            .open_store(SearchCorpusKind::LocalSymbol)
            .await
            .expect("open store");
        let table_name = service.table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                local_symbol_schema(),
                local_symbol_batches(&hits).expect("batches"),
            )
            .await
            .expect("replace record batches");
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let results = search_local_symbols(&service, "alpha", 5)
            .await
            .expect("query should succeed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AlphaSymbol");
        assert!(results[0].score > 0.0);
    }

    #[tokio::test]
    async fn local_symbol_autocomplete_reads_suggestions_from_published_epoch() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = fixture_service(&temp_dir);
        let lease = match service.coordinator().begin_build(
            SearchCorpusKind::LocalSymbol,
            "fp-2",
            SearchCorpusKind::LocalSymbol.schema_version(),
        ) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin decision: {other:?}"),
        };
        let hits = vec![
            sample_hit("AlphaSymbol", "src/lib.rs", 10),
            sample_markdown_hit("Search Design", Some("section"), None),
            sample_markdown_hit("Search Metadata", Some("property"), Some("Owner")),
        ];
        let store = service
            .open_store(SearchCorpusKind::LocalSymbol)
            .await
            .expect("open store");
        let table_name = service.table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                local_symbol_schema(),
                local_symbol_batches(&hits).expect("batches"),
            )
            .await
            .expect("replace record batches");
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let results = autocomplete_local_symbols(&service, "se", 5)
            .await
            .expect("autocomplete should succeed");

        assert_eq!(
            results
                .into_iter()
                .map(|item| (item.text, item.suggestion_type))
                .collect::<Vec<_>>(),
            vec![
                ("Search Design".to_string(), "heading".to_string()),
                ("Search Metadata".to_string(), "metadata".to_string()),
            ]
        );
    }
}
