use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
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
        return Ok(Json(SymbolSearchResponse {
            query: query_text.to_string(),
            hit_count: 0,
            selected_scope: "project".to_string(),
            partial: true,
            indexing_state: Some(status.phase.as_str().to_string()),
            index_error: status.last_error,
            hits: Vec::new(),
        }));
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

    Ok(Json(SymbolSearchResponse {
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
    }))
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
