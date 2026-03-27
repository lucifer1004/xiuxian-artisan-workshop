use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::arrow_transport::{
    arrow_payload_response, build_arrow_search_ipc, encode_json,
};
use crate::gateway::studio::symbol_index::SymbolIndexPhase;
use crate::gateway::studio::types::{
    StudioNavigationTarget, SymbolSearchHit, SymbolSearchResponse, UiProjectConfig,
};

use super::super::project_scope::project_metadata_for_path;
use super::queries::SymbolSearchQuery;

pub async fn search_symbols(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SymbolSearchQuery>,
) -> Result<Json<SymbolSearchResponse>, StudioApiError> {
    let response = load_symbol_search_response(state.as_ref(), query).await?;
    Ok(Json(response))
}

pub async fn search_symbols_hits_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SymbolSearchQuery>,
) -> Result<Response, StudioApiError> {
    let response = load_symbol_search_response(state.as_ref(), query).await?;
    symbol_hits_arrow_response(&response.hits)
        .map_err(|error| StudioApiError::internal("SEARCH_SYMBOL_HITS_ARROW_FAILED", error, None))
}

pub(crate) async fn load_symbol_search_response(
    state: &GatewayState,
    query: SymbolSearchQuery,
) -> Result<SymbolSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Symbol search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let status = state.studio.symbol_index_status()?;
    let Some(index) = state.studio.current_symbol_index() else {
        return Ok(SymbolSearchResponse {
            query: query_text.to_string(),
            hit_count: 0,
            selected_scope: "project".to_string(),
            partial: true,
            indexing_state: Some(status.phase.as_str().to_string()),
            index_error: status.last_error,
            hits: Vec::new(),
        });
    };
    let projects = state.studio.configured_projects();
    let glob_matcher = build_project_glob_matcher(projects.as_slice());
    let mut hits: Vec<SymbolSearchHit> = index
        .search_unified(query_text, limit)
        .into_iter()
        .enumerate()
        .map(|(rank, symbol)| {
            symbol_search_hit(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
                symbol,
                rank,
            )
        })
        .filter(|hit| {
            glob_matcher
                .as_ref()
                .is_none_or(|matcher| matcher.is_match(hit.path.as_str()))
        })
        .collect();
    hits.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });

    Ok(SymbolSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        selected_scope: "project".to_string(),
        partial: false,
        indexing_state: Some(SymbolIndexPhase::Ready.as_str().to_string()),
        index_error: None,
        hits: {
            hits.truncate(limit);
            hits
        },
    })
}

pub(crate) fn symbol_hits_arrow_response(hits: &[SymbolSearchHit]) -> Result<Response, String> {
    encode_symbol_hits_ipc(hits).map(arrow_payload_response)
}

fn encode_symbol_hits_ipc(hits: &[SymbolSearchHit]) -> Result<Vec<u8>, String> {
    let names: Vec<&str> = hits.iter().map(|hit| hit.name.as_str()).collect();
    let kinds: Vec<&str> = hits.iter().map(|hit| hit.kind.as_str()).collect();
    let paths: Vec<&str> = hits.iter().map(|hit| hit.path.as_str()).collect();
    let lines: Vec<u64> = hits.iter().map(|hit| hit.line as u64).collect();
    let locations: Vec<&str> = hits.iter().map(|hit| hit.location.as_str()).collect();
    let languages: Vec<&str> = hits.iter().map(|hit| hit.language.as_str()).collect();
    let sources: Vec<&str> = hits.iter().map(|hit| hit.source.as_str()).collect();
    let crate_names: Vec<&str> = hits.iter().map(|hit| hit.crate_name.as_str()).collect();
    let project_names: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.project_name.as_deref()).collect();
    let root_labels: Vec<Option<&str>> = hits.iter().map(|hit| hit.root_label.as_deref()).collect();
    let navigation_targets_json: Vec<String> = hits
        .iter()
        .map(|hit| encode_json(&hit.navigation_target))
        .collect::<Result<_, _>>()?;

    let schema = Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("line", DataType::UInt64, false),
        Field::new("location", DataType::Utf8, false),
        Field::new("language", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("crateName", DataType::Utf8, false),
        Field::new("projectName", DataType::Utf8, true),
        Field::new("rootLabel", DataType::Utf8, true),
        Field::new("navigationTargetJson", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]);

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(names)),
        Arc::new(StringArray::from(kinds)),
        Arc::new(StringArray::from(paths)),
        Arc::new(UInt64Array::from(lines)),
        Arc::new(StringArray::from(locations)),
        Arc::new(StringArray::from(languages)),
        Arc::new(StringArray::from(sources)),
        Arc::new(StringArray::from(crate_names)),
        Arc::new(StringArray::from(project_names)),
        Arc::new(StringArray::from(root_labels)),
        Arc::new(StringArray::from(
            navigation_targets_json
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )),
        Arc::new(Float64Array::from(
            hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
        )),
    ];
    build_arrow_search_ipc(schema, columns)
}

fn symbol_search_hit(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    symbol: crate::unified_symbol::UnifiedSymbol,
    rank: usize,
) -> SymbolSearchHit {
    let (path, line) = parse_symbol_location(symbol.location.as_str());
    let metadata = project_metadata_for_path(project_root, config_root, projects, path.as_str());
    let source = if symbol.is_project() {
        "project".to_string()
    } else {
        "external".to_string()
    };
    let language =
        crate::gateway::studio::search::support::source_language_label(Path::new(path.as_str()))
            .unwrap_or("unknown")
            .to_string();

    SymbolSearchHit {
        name: symbol.name,
        kind: symbol.kind,
        path: path.clone(),
        line,
        location: symbol.location,
        language,
        source,
        crate_name: symbol.crate_name,
        project_name: metadata.project_name.clone(),
        root_label: metadata.root_label.clone(),
        navigation_target: StudioNavigationTarget {
            path,
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: Some(line),
            line_end: Some(line),
            column: None,
        },
        score: if rank == usize::MAX { 0.0 } else { 0.95 },
    }
}

fn parse_symbol_location(location: &str) -> (String, usize) {
    match location.rsplit_once(':') {
        Some((path, line)) => (path.to_string(), line.parse::<usize>().unwrap_or(1)),
        None => (location.to_string(), 1),
    }
}

fn build_project_glob_matcher(projects: &[UiProjectConfig]) -> Option<GlobSet> {
    let patterns = projects
        .iter()
        .flat_map(|project| project.dirs.iter())
        .filter(|dir| is_glob_pattern(dir.as_str()))
        .collect::<Vec<_>>();
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    let mut has_pattern = false;
    for pattern in patterns {
        let Ok(glob) = Glob::new(pattern.as_str()) else {
            continue;
        };
        builder.add(glob);
        has_pattern = true;
    }

    if !has_pattern {
        return None;
    }

    builder.build().ok()
}

fn is_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use arrow::ipc::reader::StreamReader;

    #[test]
    fn search_symbol_arrow_roundtrip_preserves_navigation_target() {
        let hits = vec![SymbolSearchHit {
            name: "solve".to_string(),
            kind: "function".to_string(),
            path: "src/pkg.jl".to_string(),
            line: 42,
            location: "src/pkg.jl:42".to_string(),
            language: "julia".to_string(),
            source: "project".to_string(),
            crate_name: "pkg".to_string(),
            project_name: Some("pkg".to_string()),
            root_label: Some("pkg".to_string()),
            navigation_target: StudioNavigationTarget {
                path: "pkg/src/pkg.jl".to_string(),
                category: "repo_code".to_string(),
                project_name: Some("pkg".to_string()),
                root_label: Some("pkg".to_string()),
                line: Some(42),
                line_end: Some(42),
                column: Some(1),
            },
            score: 0.91,
        }];

        let encoded =
            encode_symbol_hits_ipc(&hits).expect("symbol hit arrow encoding should succeed");
        let reader = StreamReader::try_new(Cursor::new(encoded), None)
            .expect("symbol hit stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("symbol hit stream should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("navigationTargetJson").is_some());
        assert!(batch.column_by_name("crateName").is_some());
    }
}
