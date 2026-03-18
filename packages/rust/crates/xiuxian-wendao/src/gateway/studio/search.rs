use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Component, Path};
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use regex::RegexBuilder;
use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};
use xiuxian_ast::{Lang, extract_items, get_skeleton_patterns};

use crate::dependency_indexer::{SymbolKind, extract_symbols};
use crate::link_graph::{LinkGraphDisplayHit, LinkGraphRetrievalMode, LinkGraphSearchOptions};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::router::{GatewayState, StudioApiError};
use super::types::{
    AstSearchHit, AstSearchResponse, AutocompleteResponse, AutocompleteSuggestion,
    AutocompleteSuggestionType, ReferenceSearchHit, ReferenceSearchResponse, SearchHit,
    SearchResponse, SymbolSearchHit, SymbolSearchResponse, SymbolSearchSource,
};

const DEFAULT_SEARCH_LIMIT: usize = 10;
const MAX_SEARCH_LIMIT: usize = 200;
const DEFAULT_AST_SEARCH_LIMIT: usize = 10;
const MAX_AST_SEARCH_LIMIT: usize = 200;
const DEFAULT_REFERENCE_SEARCH_LIMIT: usize = 10;
const MAX_REFERENCE_SEARCH_LIMIT: usize = 200;
const DEFAULT_SYMBOL_SEARCH_LIMIT: usize = 10;
const MAX_SYMBOL_SEARCH_LIMIT: usize = 200;
const DEFAULT_AUTOCOMPLETE_LIMIT: usize = 5;
const MAX_AUTOCOMPLETE_LIMIT: usize = 20;

#[derive(Debug, Deserialize)]
pub(super) struct SearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AutocompleteQuery {
    prefix: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SymbolSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AstSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReferenceSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) async fn search_knowledge(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT);
    let index = state.link_graph_index().await?;
    let payload = index.search_planned_payload(raw_query, limit, LinkGraphSearchOptions::default());

    let hits = payload
        .hits
        .into_iter()
        .map(|hit| SearchHit {
            stem: hit.stem,
            title: strip_option(&hit.title),
            path: hit.path,
            doc_type: hit.doc_type,
            tags: hit.tags,
            score: hit.score.max(0.0),
            best_section: strip_option(&hit.best_section),
            match_reason: strip_option(&hit.match_reason),
        })
        .collect();

    Ok(Json(SearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count: payload.hit_count,
        graph_confidence_score: Some(payload.graph_confidence_score),
        selected_mode: Some(retrieval_mode_to_string(payload.selected_mode)),
    }))
}

pub(super) async fn search_autocomplete(
    Query(query): Query<AutocompleteQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AutocompleteResponse>, StudioApiError> {
    let prefix = query
        .prefix
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PREFIX", "`prefix` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AUTOCOMPLETE_LIMIT)
        .clamp(1, MAX_AUTOCOMPLETE_LIMIT);
    let index = state.link_graph_index().await?;
    let payload =
        index.search_planned_payload(prefix, limit.max(2), LinkGraphSearchOptions::default());

    Ok(Json(AutocompleteResponse {
        prefix: prefix.to_string(),
        suggestions: collect_autocomplete_suggestions(prefix, &payload.hits, limit),
    }))
}

pub(super) async fn search_ast(
    Query(query): Query<AstSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AST_SEARCH_LIMIT)
        .clamp(1, MAX_AST_SEARCH_LIMIT);
    let index = state.studio.ast_index().await?;
    let mut hits = index
        .iter()
        .filter(|hit| ast_hit_matches(hit, raw_query))
        .map(|hit| {
            let mut hit = hit.clone();
            hit.score = score_ast_hit(&hit, raw_query);
            hit
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    hits.truncate(limit);
    let hit_count = hits.len();

    Ok(Json(AstSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "definitions".to_string(),
    }))
}

pub(super) async fn search_symbols(
    Query(query): Query<SymbolSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SymbolSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SYMBOL_SEARCH_LIMIT)
        .clamp(1, MAX_SYMBOL_SEARCH_LIMIT);
    let search_window = limit.saturating_mul(4).min(MAX_SYMBOL_SEARCH_LIMIT);
    let index = state.studio.symbol_index().await?;
    let mut hits = index
        .search_project(raw_query, search_window)
        .into_iter()
        .map(|symbol| symbol_to_hit(symbol, raw_query))
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    hits.truncate(limit);
    let hit_count = hits.len();

    Ok(Json(SymbolSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "project".to_string(),
    }))
}

pub(super) async fn search_references(
    Query(query): Query<ReferenceSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<ReferenceSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_REFERENCE_SEARCH_LIMIT)
        .clamp(1, MAX_REFERENCE_SEARCH_LIMIT);
    let ast_index = state.studio.ast_index().await?;
    let project_root = state.studio.project_root.clone();
    let query_owned = raw_query.to_string();
    let ast_hits = ast_index.as_ref().clone();
    let hits = tokio::task::spawn_blocking(move || {
        build_reference_hits(
            project_root.as_path(),
            ast_hits.as_slice(),
            query_owned.as_str(),
            limit,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_PANIC",
            "Failed to execute studio reference search",
            Some(error.to_string()),
        )
    })?
    .map_err(|error| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_FAILED",
            "Failed to execute studio reference search",
            Some(error),
        )
    })?;
    let hit_count = hits.len();

    Ok(Json(ReferenceSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "references".to_string(),
    }))
}

fn retrieval_mode_to_string(mode: LinkGraphRetrievalMode) -> String {
    match mode {
        LinkGraphRetrievalMode::GraphOnly => "graph_only".to_string(),
        LinkGraphRetrievalMode::Hybrid => "hybrid".to_string(),
        LinkGraphRetrievalMode::VectorOnly => "vector_only".to_string(),
    }
}

fn strip_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn symbol_to_hit(symbol: &crate::unified_symbol::UnifiedSymbol, query: &str) -> SymbolSearchHit {
    let (path, line) = split_location(symbol.location.as_str());

    SymbolSearchHit {
        name: symbol.name.clone(),
        kind: symbol.kind.clone(),
        path: path.clone(),
        line,
        location: symbol.location.clone(),
        language: source_language_label(Path::new(path.as_str()))
            .unwrap_or("unknown")
            .to_string(),
        crate_name: symbol.crate_or_local().to_string(),
        source: if symbol.is_project() {
            SymbolSearchSource::Project
        } else {
            SymbolSearchSource::External
        },
        score: score_symbol(symbol.name.as_str(), path.as_str(), query),
    }
}

pub(super) fn build_ast_index(project_root: &Path) -> Result<Vec<AstSearchHit>, String> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry))
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }

        let relative_path = entry
            .path()
            .strip_prefix(project_root)
            .unwrap_or_else(|_| entry.path());
        let Some(lang) = ast_search_lang(relative_path) else {
            continue;
        };

        let content = std::fs::read_to_string(entry.path())
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        let normalized_path = normalize_path(relative_path);
        let crate_name = infer_crate_name(relative_path);

        for pattern in get_skeleton_patterns(lang) {
            for result in extract_items(content.as_str(), pattern, lang, Some(vec!["NAME"])) {
                let name = result
                    .captures
                    .get("NAME")
                    .cloned()
                    .unwrap_or_else(|| first_signature_line(result.text.as_str()).to_string());
                let signature = first_signature_line(result.text.as_str()).to_string();
                if signature.is_empty() {
                    continue;
                }
                let dedupe_key = format!(
                    "{normalized_path}:{}:{}:{name}",
                    result.line_start, result.line_end
                );
                if !seen.insert(dedupe_key) {
                    continue;
                }

                hits.push(AstSearchHit {
                    name,
                    signature,
                    path: normalized_path.clone(),
                    language: lang.as_str().to_string(),
                    crate_name: crate_name.clone(),
                    line_start: result.line_start,
                    line_end: result.line_end,
                    score: 0.0,
                });
            }
        }
    }

    Ok(hits)
}

pub(super) fn build_symbol_index(project_root: &Path) -> Result<UnifiedSymbolIndex, String> {
    let mut index = UnifiedSymbolIndex::new();

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry))
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }

        let Some(language) = source_language_label(entry.path()) else {
            continue;
        };
        let relative_path = entry
            .path()
            .strip_prefix(project_root)
            .unwrap_or_else(|_| entry.path());
        let normalized_path = normalize_path(relative_path);
        let crate_name = infer_crate_name(relative_path);
        let symbols = extract_symbols(entry.path(), language)
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;

        for symbol in symbols {
            let location = format!("{normalized_path}:{}", symbol.line);
            index.add_project_symbol(
                symbol.name.as_str(),
                symbol_kind_label(&symbol.kind),
                location.as_str(),
                crate_name.as_str(),
            );
        }
    }

    Ok(index)
}

fn build_reference_hits(
    project_root: &Path,
    ast_hits: &[AstSearchHit],
    query: &str,
    limit: usize,
) -> Result<Vec<ReferenceSearchHit>, String> {
    let regex = RegexBuilder::new(format!(r"\b{}\b", regex::escape(query)).as_str())
        .case_insensitive(true)
        .build()
        .map_err(|error| error.to_string())?;
    let definition_locations = ast_hits
        .iter()
        .filter(|hit| hit.name.eq_ignore_ascii_case(query))
        .map(|hit| (hit.path.clone(), hit.line_start))
        .collect::<HashSet<_>>();

    let mut hits = Vec::new();

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry))
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }

        let relative_path = entry
            .path()
            .strip_prefix(project_root)
            .unwrap_or_else(|_| entry.path());
        let Some(lang) = ast_search_lang(relative_path) else {
            continue;
        };
        let normalized_path = normalize_path(relative_path);
        let crate_name = infer_crate_name(relative_path);
        let content = std::fs::read_to_string(entry.path())
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;

        for (line_idx, line_text) in content.lines().enumerate() {
            let line_number = line_idx + 1;
            if definition_locations.contains(&(normalized_path.clone(), line_number)) {
                continue;
            }

            let Some(mat) = regex.find(line_text) else {
                continue;
            };

            hits.push(ReferenceSearchHit {
                name: query.to_string(),
                path: normalized_path.clone(),
                language: lang.as_str().to_string(),
                crate_name: crate_name.clone(),
                line: line_number,
                column: line_text[..mat.start()].chars().count() + 1,
                line_text: line_text.trim().to_string(),
                score: score_reference_hit(line_text, query),
            });
        }
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
    });
    hits.truncate(limit);

    Ok(hits)
}

struct AutocompleteCollector<'a> {
    suggestions: Vec<AutocompleteSuggestion>,
    seen: HashSet<String>,
    prefix_lc: &'a str,
    limit: usize,
}

fn should_skip_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git"
            | ".cache"
            | ".devenv"
            | ".direnv"
            | ".run"
            | "target"
            | "node_modules"
            | "dist"
            | "coverage"
            | "__pycache__"
    )
}

fn ast_search_lang(path: &Path) -> Option<Lang> {
    match Lang::from_path(path)? {
        Lang::Python
        | Lang::Rust
        | Lang::JavaScript
        | Lang::TypeScript
        | Lang::Bash
        | Lang::Go
        | Lang::Java
        | Lang::C
        | Lang::Cpp
        | Lang::CSharp
        | Lang::Ruby
        | Lang::Swift
        | Lang::Kotlin
        | Lang::Lua
        | Lang::Php => Lang::from_path(path),
        _ => None,
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn infer_crate_name(relative_path: &Path) -> String {
    let components = relative_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();

    match components.as_slice() {
        [packages, rust, crates, crate_name, ..]
            if packages == "packages" && rust == "rust" && crates == "crates" =>
        {
            crate_name.clone()
        }
        [packages, python, package_name, ..] if packages == "packages" && python == "python" => {
            package_name.clone()
        }
        [data, workspace_name, ..] if data == ".data" => workspace_name.clone(),
        [skills, skill_name, ..] if skills == "internal_skills" => skill_name.clone(),
        [first, ..] => first.clone(),
        [] => "workspace".to_string(),
    }
}

fn source_language_label(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Some("rust"),
        Some("py") => Some("python"),
        _ => None,
    }
}

fn split_location(location: &str) -> (String, usize) {
    match location.rsplit_once(':') {
        Some((path, line)) => (
            path.to_string(),
            line.parse::<usize>().unwrap_or_default().max(1),
        ),
        None => (location.to_string(), 1),
    }
}

fn first_signature_line(text: &str) -> &str {
    text.lines().next().map(str::trim).unwrap_or_default()
}

fn ast_hit_matches(hit: &AstSearchHit, query: &str) -> bool {
    let query_lc = query.to_ascii_lowercase();
    hit.name.to_ascii_lowercase().contains(query_lc.as_str())
        || hit
            .signature
            .to_ascii_lowercase()
            .contains(query_lc.as_str())
        || hit.path.to_ascii_lowercase().contains(query_lc.as_str())
        || hit
            .language
            .to_ascii_lowercase()
            .contains(query_lc.as_str())
        || hit
            .crate_name
            .to_ascii_lowercase()
            .contains(query_lc.as_str())
}

fn score_ast_hit(hit: &AstSearchHit, query: &str) -> f64 {
    let query_lc = query.to_ascii_lowercase();
    let name_lc = hit.name.to_ascii_lowercase();
    let signature_lc = hit.signature.to_ascii_lowercase();
    let path_lc = hit.path.to_ascii_lowercase();

    if name_lc == query_lc {
        1.0
    } else if name_lc.starts_with(query_lc.as_str()) {
        0.95
    } else if name_lc.contains(query_lc.as_str()) {
        0.88
    } else if signature_lc.contains(query_lc.as_str()) {
        0.8
    } else if path_lc.contains(query_lc.as_str()) {
        0.72
    } else {
        0.5
    }
}

fn score_reference_hit(line_text: &str, query: &str) -> f64 {
    let normalized_line = line_text.trim();
    if normalized_line.contains(query) {
        0.9
    } else if normalized_line
        .to_ascii_lowercase()
        .contains(query.to_ascii_lowercase().as_str())
    {
        0.82
    } else {
        0.7
    }
}

fn score_symbol(name: &str, path: &str, query: &str) -> f64 {
    let name_lc = name.to_ascii_lowercase();
    let path_lc = path.to_ascii_lowercase();
    let query_lc = query.to_ascii_lowercase();

    if name_lc == query_lc {
        1.0
    } else if name_lc.starts_with(query_lc.as_str()) {
        0.95
    } else if name_lc.contains(query_lc.as_str()) {
        0.88
    } else if path_lc.contains(query_lc.as_str()) {
        0.72
    } else {
        0.5
    }
}

fn symbol_kind_label(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Trait => "trait",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Field => "field",
        SymbolKind::Impl => "impl",
        SymbolKind::Mod => "module",
        SymbolKind::Const => "const",
        SymbolKind::Static => "static",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Unknown => "unknown",
    }
}

impl<'a> AutocompleteCollector<'a> {
    fn new(prefix_lc: &'a str, limit: usize) -> Self {
        Self {
            suggestions: Vec::with_capacity(limit),
            seen: HashSet::new(),
            prefix_lc,
            limit,
        }
    }

    fn add(
        &mut self,
        text: &str,
        path: &str,
        doc_type: Option<&str>,
        suggestion_type: AutocompleteSuggestionType,
    ) {
        if self.suggestions.len() >= self.limit {
            return;
        }

        let normalized_text = text.trim();
        if normalized_text.is_empty()
            || !normalized_text
                .to_ascii_lowercase()
                .starts_with(self.prefix_lc)
        {
            return;
        }

        let key = format!("{suggestion_type:?}|{normalized_text}|{path}");
        if !self.seen.insert(key) {
            return;
        }

        self.suggestions.push(AutocompleteSuggestion {
            text: normalized_text.to_string(),
            suggestion_type,
            path: Some(path.to_string()),
            doc_type: doc_type.map(ToString::to_string),
        });
    }
}

fn collect_autocomplete_suggestions(
    prefix: &str,
    hits: &[LinkGraphDisplayHit],
    limit: usize,
) -> Vec<AutocompleteSuggestion> {
    let prefix_lc = prefix.to_ascii_lowercase();
    let mut collector = AutocompleteCollector::new(&prefix_lc, limit);

    for hit in hits {
        collector.add(
            &hit.stem,
            hit.path.as_str(),
            hit.doc_type.as_deref(),
            AutocompleteSuggestionType::Stem,
        );

        if !hit.title.is_empty() {
            collector.add(
                &hit.title,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Title,
            );
        }

        for tag in &hit.tags {
            collector.add(
                tag,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Tag,
            );
        }

        if collector.suggestions.len() >= limit {
            break;
        }
    }

    collector.suggestions
}

#[cfg(test)]
#[path = "../../../tests/unit/gateway/studio/search.rs"]
mod tests;
