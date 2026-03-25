use std::collections::HashSet;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, LanceUInt64Array,
    VectorStore, VectorStoreError,
};

use crate::gateway::studio::types::{AstSearchHit, AutocompleteSuggestion};
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, sort_by_rank, trim_ranked_vec,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{hit_json_column, projected_columns, suggestion_columns};

const MIN_RETAINED_LOCAL_SYMBOLS: usize = 64;
const RETAINED_LOCAL_SYMBOL_MULTIPLIER: usize = 4;
const MIN_RETAINED_AUTOCOMPLETE_SUGGESTIONS: usize = 16;
const RETAINED_AUTOCOMPLETE_MULTIPLIER: usize = 2;

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
    let query_lower = query.trim().to_ascii_lowercase();
    if query_lower.is_empty() {
        return Ok(Vec::new());
    }

    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if table_names.is_empty() {
        return Ok(Vec::new());
    }
    let mut columns = projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    columns.push(hit_json_column().to_string());
    let window = retained_window(limit);
    let execution = execute_local_symbol_search(
        &store,
        table_names.as_slice(),
        query_lower.as_str(),
        ColumnarScanOptions {
            projected_columns: columns,
            batch_size: Some(256),
            limit: Some(limit.saturating_mul(32).max(128)),
            ..ColumnarScanOptions::default()
        },
        window,
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits = decode_local_symbol_hits(candidates)?;
    service.record_query_telemetry(
        SearchCorpusKind::LocalSymbol,
        execution
            .telemetry
            .finish(execution.source, Some("search".to_string()), hits.len()),
    );
    Ok(hits)
}

#[derive(Debug)]
struct LocalSymbolSearchExecution {
    candidates: Vec<LocalSymbolCandidate>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

async fn execute_local_symbol_search(
    store: &VectorStore,
    table_names: &[String],
    query_lower: &str,
    options: ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<LocalSymbolSearchExecution, LocalSymbolSearchError> {
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut candidates = Vec::with_capacity(window.target);
    store
        .scan_record_batches_streaming_across_tables(table_names, options, |_table_name, batch| {
            collect_candidates(&batch, query_lower, &mut candidates, window, &mut telemetry)
        })
        .await?;
    Ok(LocalSymbolSearchExecution {
        candidates,
        telemetry,
        source: StreamingRerankSource::Scan,
    })
}

fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(
        limit,
        RETAINED_LOCAL_SYMBOL_MULTIPLIER,
        MIN_RETAINED_LOCAL_SYMBOLS,
    )
}

fn suggestion_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(
        limit,
        RETAINED_AUTOCOMPLETE_MULTIPLIER,
        MIN_RETAINED_AUTOCOMPLETE_SUGGESTIONS,
    )
}

fn decode_local_symbol_hits(
    candidates: Vec<LocalSymbolCandidate>,
) -> Result<Vec<AstSearchHit>, LocalSymbolSearchError> {
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
    let table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if table_names.is_empty() {
        return Ok(Vec::new());
    }
    let execution = execute_local_symbol_autocomplete(
        &store,
        table_names.as_slice(),
        normalized_prefix.as_str(),
        ColumnarScanOptions {
            projected_columns: suggestion_columns()
                .into_iter()
                .map(str::to_string)
                .collect(),
            batch_size: Some(256),
            limit: Some(limit.saturating_mul(64).max(256)),
            ..ColumnarScanOptions::default()
        },
        suggestion_window(limit),
    )
    .await?;
    let mut suggestions = execution.suggestions;
    suggestions.sort_by(|left, right| compare_suggestions(left, right));
    suggestions.truncate(limit);
    service.record_query_telemetry(
        SearchCorpusKind::LocalSymbol,
        execution.telemetry.finish(
            execution.source,
            Some("autocomplete".to_string()),
            suggestions.len(),
        ),
    );
    Ok(suggestions)
}

struct LocalSymbolAutocompleteExecution {
    suggestions: Vec<AutocompleteSuggestion>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

async fn execute_local_symbol_autocomplete(
    store: &VectorStore,
    table_names: &[String],
    normalized_prefix: &str,
    options: ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<LocalSymbolAutocompleteExecution, LocalSymbolSearchError> {
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut suggestions = Vec::with_capacity(window.target);
    let mut seen = HashSet::new();
    store
        .scan_record_batches_streaming_across_tables(table_names, options, |_table_name, batch| {
            collect_suggestions(
                &batch,
                normalized_prefix,
                &mut suggestions,
                &mut seen,
                window,
                &mut telemetry,
            )
        })
        .await?;
    Ok(LocalSymbolAutocompleteExecution {
        suggestions,
        telemetry,
        source: StreamingRerankSource::Scan,
    })
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
    candidates: &mut Vec<LocalSymbolCandidate>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), LocalSymbolSearchError> {
    telemetry.observe_batch(batch.num_rows());
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
            if owner_title.is_null(row) {
                ""
            } else {
                owner_title.value(row)
            },
        );
        if score <= 0.0 {
            continue;
        }

        telemetry.observe_match();
        candidates.push(LocalSymbolCandidate {
            score,
            name: name.value(row).to_string(),
            path: path.value(row).to_string(),
            line_start: usize::try_from(line_start.value(row)).unwrap_or(usize::MAX),
            hit_json: hit_json.value(row).to_string(),
        });
        telemetry.observe_working_set(candidates.len());
        if candidates.len() > window.threshold {
            let before_len = candidates.len();
            trim_ranked_vec(candidates, window.target, compare_candidates);
            telemetry.observe_trim(before_len, candidates.len());
        }
    }

    Ok(())
}

fn compare_candidates(
    left: &LocalSymbolCandidate,
    right: &LocalSymbolCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.line_start.cmp(&right.line_start))
}

fn collect_suggestions(
    batch: &LanceRecordBatch,
    normalized_prefix: &str,
    suggestions: &mut Vec<AutocompleteSuggestion>,
    seen: &mut HashSet<String>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), LocalSymbolSearchError> {
    telemetry.observe_batch(batch.num_rows());
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

        telemetry.observe_match();
        suggestions.push(AutocompleteSuggestion {
            text: text.to_string(),
            suggestion_type: autocomplete_suggestion_type(
                language.value(row),
                nullable_string_value(node_kind, row),
            )
            .to_string(),
        });
        telemetry.observe_working_set(suggestions.len());
        if suggestions.len() > window.threshold {
            let before_len = suggestions.len();
            trim_ranked_vec(suggestions, window.target, compare_suggestions);
            telemetry.observe_trim(before_len, suggestions.len());
        }
    }

    Ok(())
}

fn compare_suggestions(
    left: &AutocompleteSuggestion,
    right: &AutocompleteSuggestion,
) -> std::cmp::Ordering {
    suggestion_rank(left)
        .cmp(&suggestion_rank(right))
        .then_with(|| left.text.cmp(&right.text))
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

fn nullable_string_value(array: &LanceStringArray, row: usize) -> Option<&str> {
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
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
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
            .unwrap_or_else(|error| panic!("open store: {error}"));
        let table_name = SearchPlaneService::table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                local_symbol_schema(),
                local_symbol_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replace record batches: {error}"));
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let results = search_local_symbols(&service, "alpha", 5)
            .await
            .unwrap_or_else(|error| panic!("query should succeed: {error}"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AlphaSymbol");
        assert!(results[0].score > 0.0);

        let snapshot = service.status();
        let corpus = snapshot
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::LocalSymbol)
            .unwrap_or_else(|| panic!("local symbol corpus row should exist"));
        let telemetry = corpus
            .last_query_telemetry
            .as_ref()
            .unwrap_or_else(|| panic!("local symbol telemetry should be present"));
        assert_eq!(
            telemetry.source,
            crate::search_plane::SearchQueryTelemetrySource::Scan
        );
        assert_eq!(telemetry.scope.as_deref(), Some("search"));
        assert!(telemetry.rows_scanned >= 1);
        assert!(telemetry.matched_rows >= 1);
    }

    #[tokio::test]
    async fn local_symbol_query_can_rerank_across_multiple_tables() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let service = fixture_service(&temp_dir);
        let store = service
            .open_store(SearchCorpusKind::LocalSymbol)
            .await
            .unwrap_or_else(|error| panic!("open store: {error}"));
        let hits_a = vec![sample_hit("AlphaSymbol", "src/lib.rs", 10)];
        let hits_b = vec![sample_hit("BetaAlphaHelper", "src/beta.rs", 20)];

        store
            .replace_record_batches(
                "local_symbol_project_a",
                local_symbol_schema(),
                local_symbol_batches(&hits_a).unwrap_or_else(|error| panic!("batches a: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replace record batches a: {error}"));
        store
            .replace_record_batches(
                "local_symbol_project_b",
                local_symbol_schema(),
                local_symbol_batches(&hits_b).unwrap_or_else(|error| panic!("batches b: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replace record batches b: {error}"));

        let execution = execute_local_symbol_search(
            &store,
            &[
                "local_symbol_project_a".to_string(),
                "local_symbol_project_b".to_string(),
            ],
            "alpha",
            ColumnarScanOptions {
                projected_columns: projected_columns()
                    .into_iter()
                    .map(str::to_string)
                    .chain(std::iter::once(hit_json_column().to_string()))
                    .collect(),
                batch_size: Some(64),
                ..ColumnarScanOptions::default()
            },
            retained_window(5),
        )
        .await
        .unwrap_or_else(|error| panic!("multi-table query should succeed: {error}"));

        let hits = decode_local_symbol_hits(execution.candidates)
            .unwrap_or_else(|error| panic!("decode hits should succeed: {error}"));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "AlphaSymbol");
        assert_eq!(hits[1].name, "BetaAlphaHelper");
    }

    #[tokio::test]
    async fn local_symbol_autocomplete_reads_suggestions_from_published_epoch() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
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
            .unwrap_or_else(|error| panic!("open store: {error}"));
        let table_name = SearchPlaneService::table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                local_symbol_schema(),
                local_symbol_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replace record batches: {error}"));
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let results = autocomplete_local_symbols(&service, "se", 5)
            .await
            .unwrap_or_else(|error| panic!("autocomplete should succeed: {error}"));

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

        let snapshot = service.status();
        let corpus = snapshot
            .corpora
            .iter()
            .find(|entry| entry.corpus == SearchCorpusKind::LocalSymbol)
            .unwrap_or_else(|| panic!("local symbol corpus row should exist"));
        let telemetry = corpus
            .last_query_telemetry
            .as_ref()
            .unwrap_or_else(|| panic!("autocomplete telemetry should be present"));
        assert_eq!(
            telemetry.source,
            crate::search_plane::SearchQueryTelemetrySource::Scan
        );
        assert_eq!(telemetry.scope.as_deref(), Some("autocomplete"));
        assert!(telemetry.rows_scanned >= 1);
        assert!(telemetry.matched_rows >= 2);
    }
}
