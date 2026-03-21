//! Search backend integration for Studio API.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::analyzers::{
    ExampleSearchQuery, ModuleSearchQuery, SymbolSearchQuery as RepoSymbolSearchQuery,
    build_example_search, build_module_search, build_symbol_search,
};
use crate::gateway::studio::repo_index::{RepoIndexPhase, RepoIndexSnapshot};
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::types::{
    AstSearchHit, AstSearchResponse, AttachmentSearchHit, AttachmentSearchResponse,
    AutocompleteResponse, AutocompleteSuggestion, DefinitionResolveResponse,
    ReferenceSearchResponse, SearchBacklinkItem, SearchHit, SearchResponse, StudioNavigationTarget,
    SymbolSearchHit, SymbolSearchResponse, UiProjectConfig,
};
use crate::link_graph::LinkGraphAttachmentKind;
use crate::unified_symbol::UnifiedSymbolIndex;

use super::definition::{
    DefinitionMatchMode, DefinitionResolveOptions, resolve_best_definition,
    resolve_definition_candidates,
};
use super::observation_hints::definition_observation_hints;
use super::project_scope::project_metadata_for_path;
use super::source_index;

pub fn build_ast_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<Vec<AstSearchHit>, String> {
    source_index::build_ast_index(project_root, config_root, projects)
}

#[cfg(test)]
#[path = "../../../../tests/unit/gateway/studio/search.rs"]
mod studio_search_tests;

pub fn build_symbol_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<UnifiedSymbolIndex, String> {
    source_index::build_symbol_index(project_root, config_root, projects)
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(alias = "query")]
    pub q: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DefinitionResolveQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub line: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AttachmentSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub ext: Vec<String>,
    #[serde(default)]
    pub kind: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub struct AutocompleteQuery {
    pub prefix: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AstSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SymbolSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn search_knowledge(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Knowledge search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(10).max(1);
    let response = build_knowledge_search_response(
        state.studio.as_ref(),
        query_text,
        limit,
        query
            .intent
            .clone()
            .or_else(|| Some("semantic_lookup".to_string())),
    )
    .await?;
    Ok(Json(response))
}

pub async fn search_intent(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    let intent = query.intent.clone().unwrap_or_default();
    let limit = query.limit.unwrap_or(10).max(1);

    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Intent search requires a non-empty query",
        ));
    }

    if intent == "code_search" {
        let response =
            build_code_search_response(state.studio.as_ref(), raw_query, query.repo, limit)?;
        return Ok(Json(response));
    }

    let response = build_knowledge_search_response(
        state.studio.as_ref(),
        query_text,
        limit,
        (!intent.is_empty()).then_some(intent),
    )
    .await?;
    Ok(Json(response))
}

pub async fn search_attachments(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AttachmentSearchQuery>,
) -> Result<Json<AttachmentSearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Attachment search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let projects = state.studio.configured_projects();
    let graph_index = state.link_graph_index().await?;
    let extensions = query
        .ext
        .iter()
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let kinds = query
        .kind
        .iter()
        .map(|value| LinkGraphAttachmentKind::from_alias(value))
        .collect::<Vec<_>>();
    let hits = graph_index
        .search_attachments(
            query_text,
            limit,
            extensions.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .into_iter()
        .map(|hit| {
            attachment_search_hit(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
                hit,
            )
        })
        .collect::<Vec<_>>();

    Ok(Json(AttachmentSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "attachments".to_string(),
    }))
}

pub async fn search_ast(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AstSearchQuery>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "AST search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let ast_index = state.studio.ast_index().await?;
    let projects = state.studio.configured_projects();
    let mut hits = ast_index
        .iter()
        .filter(|hit| ast_hit_matches(hit, query_text))
        .map(|hit| {
            enrich_ast_hit(
                hit,
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    hits.truncate(limit);

    Ok(Json(AstSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "definitions".to_string(),
    }))
}

pub async fn search_definition(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<DefinitionResolveQuery>,
) -> Result<Json<DefinitionResolveResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Definition search requires a non-empty query",
        ));
    }

    let source_path = query
        .path
        .as_deref()
        .map(|path| normalize_source_path(state.studio.project_root.as_path(), path));
    let source_paths = source_path
        .as_ref()
        .map(|path| std::slice::from_ref(path))
        .filter(|paths| !paths.is_empty());
    let observation_hints =
        definition_observation_hints(state.as_ref(), source_paths, query.line, query_text).await;
    let ast_index = state.studio.ast_index().await?;
    let projects = state.studio.configured_projects();
    let options = DefinitionResolveOptions {
        scope_patterns: observation_hints.as_ref().and_then(|hints| {
            (!hints.scope_patterns.is_empty()).then_some(hints.scope_patterns.clone())
        }),
        languages: observation_hints
            .as_ref()
            .and_then(|hints| (!hints.languages.is_empty()).then_some(hints.languages.clone())),
        preferred_source_path: source_path.clone(),
        match_mode: DefinitionMatchMode::ExactOnly,
        include_markdown: false,
        ..DefinitionResolveOptions::default()
    };
    let candidates = resolve_definition_candidates(
        query_text,
        ast_index.as_slice(),
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        projects.as_slice(),
        &options,
    );
    let Some(definition) = resolve_best_definition(
        query_text,
        ast_index.as_slice(),
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        projects.as_slice(),
        &options,
    ) else {
        return Err(StudioApiError::not_found("Definition not found"));
    };
    let navigation_target = definition.navigation_target.clone();

    Ok(Json(DefinitionResolveResponse {
        query: query_text.to_string(),
        source_path,
        source_line: query.line,
        candidate_count: candidates.len(),
        selected_scope: "definition".to_string(),
        navigation_target: navigation_target.clone(),
        definition: definition.clone(),
        resolved_target: Some(navigation_target),
        resolved_hit: Some(definition),
    }))
}

pub async fn search_references(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<ReferenceSearchQuery>,
) -> Result<Json<ReferenceSearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Reference search requires a non-empty query",
        ));
    }

    let ast_index = state.studio.ast_index().await?;
    let projects = state.studio.configured_projects();
    let hits = source_index::build_reference_hits(
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        projects.as_slice(),
        ast_index.as_slice(),
        query_text,
        query.limit.unwrap_or(20).max(1),
    )
    .map_err(|detail| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_BUILD_FAILED",
            "Failed to build Studio reference search results",
            Some(detail),
        )
    })?;

    Ok(Json(ReferenceSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "references".to_string(),
    }))
}

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
    let index = state.studio.symbol_index().await?;
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
        hits: {
            hits.truncate(limit);
            hits
        },
    }))
}

pub async fn search_autocomplete(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AutocompleteQuery>,
) -> Result<Json<AutocompleteResponse>, StudioApiError> {
    let prefix = query.prefix.unwrap_or_default().trim().to_string();
    let limit = query.limit.unwrap_or(8).max(1);
    let suggestions = if prefix.is_empty() {
        Vec::new()
    } else {
        let ast_index = state.studio.ast_index().await?;
        build_autocomplete_suggestions(ast_index.as_slice(), prefix.as_str(), limit)
    };

    Ok(Json(AutocompleteResponse {
        prefix,
        suggestions,
    }))
}

fn build_autocomplete_suggestions(
    ast_hits: &[AstSearchHit],
    prefix: &str,
    limit: usize,
) -> Vec<AutocompleteSuggestion> {
    let normalized_prefix = prefix.to_ascii_lowercase();
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();

    for suggestion in ast_hits
        .iter()
        .filter_map(|hit| autocomplete_suggestion_from_ast(hit, normalized_prefix.as_str()))
    {
        let dedupe_key = suggestion.text.to_ascii_lowercase();
        if seen.insert(dedupe_key) {
            suggestions.push(suggestion);
        }
    }

    suggestions.sort_by(|left, right| {
        autocomplete_suggestion_rank(left)
            .cmp(&autocomplete_suggestion_rank(right))
            .then_with(|| left.text.cmp(&right.text))
    });
    suggestions.truncate(limit);
    suggestions
}

fn autocomplete_suggestion_from_ast(
    hit: &AstSearchHit,
    normalized_prefix: &str,
) -> Option<AutocompleteSuggestion> {
    let text = hit.name.trim();
    if text.is_empty() || !autocomplete_matches_prefix(text, normalized_prefix) {
        return None;
    }

    Some(AutocompleteSuggestion {
        text: text.to_string(),
        suggestion_type: autocomplete_suggestion_type(hit).to_string(),
    })
}

fn autocomplete_matches_prefix(text: &str, normalized_prefix: &str) -> bool {
    let normalized_text = text.to_ascii_lowercase();
    if normalized_text.starts_with(normalized_prefix) {
        return true;
    }

    normalized_text
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| !token.is_empty() && token.starts_with(normalized_prefix))
}

fn autocomplete_suggestion_type(hit: &AstSearchHit) -> &'static str {
    if hit.language == "markdown" {
        match hit.node_kind.as_deref() {
            Some("property") | Some("observation") => "metadata",
            _ => "heading",
        }
    } else {
        "symbol"
    }
}

fn autocomplete_suggestion_rank(suggestion: &AutocompleteSuggestion) -> usize {
    match suggestion.suggestion_type.as_str() {
        "symbol" => 0,
        "heading" => 1,
        "metadata" => 2,
        _ => 3,
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ParsedCodeSearchQuery {
    query: String,
    repo: Option<String>,
    languages: Vec<String>,
    kinds: Vec<String>,
}

const CODE_CONTENT_EXTENSIONS: [&str; 4] = ["jl", "julia", "mo", "modelica"];
const CODE_CONTENT_EXCLUDE_GLOBS: [&str; 7] = [
    ".git/**",
    ".cache/**",
    ".devenv/**",
    ".direnv/**",
    "node_modules/**",
    "target/**",
    "dist/**",
];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ParsedRepoCodeSearchQuery {
    language_filters: HashSet<String>,
    kind_filters: HashSet<String>,
    search_term: Option<String>,
}

impl ParsedRepoCodeSearchQuery {
    fn search_term(&self) -> Option<&str> {
        self.search_term.as_deref()
    }
}

async fn build_knowledge_search_response(
    studio: &crate::gateway::studio::router::StudioState,
    query_text: &str,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchResponse, StudioApiError> {
    let graph_index = studio.graph_index().await?;
    let projects = studio.configured_projects();
    let hits = graph_index
        .execute_search(
            query_text,
            limit,
            &crate::link_graph::LinkGraphSearchOptions::default(),
        )
        .into_iter()
        .map(|hit| {
            knowledge_graph_hit_to_search_hit(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                projects.as_slice(),
                hit,
            )
        })
        .collect::<Vec<_>>();

    let selected_mode = if hits.is_empty() {
        "vector_only".to_string()
    } else {
        "graph_fts".to_string()
    };
    let graph_confidence_score = if hits.is_empty() { 0.0 } else { 1.0 };

    Ok(SearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        graph_confidence_score: Some(graph_confidence_score),
        selected_mode: Some(selected_mode.clone()),
        intent,
        intent_confidence: Some(graph_confidence_score),
        search_mode: Some(selected_mode),
        partial: false,
        indexing_state: None,
        pending_repos: Vec::new(),
        skipped_repos: Vec::new(),
    })
}

fn normalize_source_path(project_root: &Path, path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.strip_prefix(project_root).map_or_else(
            |_| path.to_string_lossy().replace('\\', "/"),
            |relative| relative.to_string_lossy().replace('\\', "/"),
        );
    }

    path.to_string_lossy().replace('\\', "/")
}

fn ast_hit_matches(hit: &AstSearchHit, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    hit.name.to_ascii_lowercase().contains(query.as_str())
        || hit.signature.to_ascii_lowercase().contains(query.as_str())
        || hit
            .owner_title
            .as_deref()
            .is_some_and(|owner| owner.to_ascii_lowercase().contains(query.as_str()))
}

fn enrich_ast_hit(
    hit: &AstSearchHit,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> AstSearchHit {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    let mut navigation_target = hit.navigation_target.clone();
    navigation_target.project_name = metadata.project_name.clone();
    navigation_target.root_label = metadata.root_label.clone();

    let mut enriched = hit.clone();
    enriched.project_name = metadata.project_name;
    enriched.root_label = metadata.root_label;
    enriched.navigation_target = navigation_target;
    enriched.score = ast_hit_score(&enriched);
    enriched
}

fn ast_hit_score(hit: &AstSearchHit) -> f64 {
    if hit.language != "markdown" {
        return 0.95;
    }

    match hit.node_kind.as_deref() {
        Some("task") => 0.88,
        Some("property") | Some("observation") => 0.8,
        _ => 0.95,
    }
}

fn attachment_search_hit(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    hit: crate::link_graph::LinkGraphAttachmentHit,
) -> AttachmentSearchHit {
    let metadata = project_metadata_for_path(
        project_root,
        config_root,
        projects,
        hit.source_path.as_str(),
    );

    AttachmentSearchHit {
        name: hit.attachment_name.clone(),
        path: hit.source_path.clone(),
        navigation_target: StudioNavigationTarget {
            path: hit.source_path.clone(),
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: None,
            line_end: None,
            column: None,
        },
        score: hit.score,
        source_id: hit.source_id.clone(),
        source_stem: hit.source_stem,
        source_title: hit.source_title,
        source_path: hit.source_path,
        attachment_id: format!("att://{}/{}", hit.source_id, hit.attachment_path),
        attachment_path: hit.attachment_path,
        attachment_name: hit.attachment_name,
        attachment_ext: hit.attachment_ext,
        kind: attachment_kind_label(hit.kind).to_string(),
        vision_snippet: hit.vision_snippet,
    }
}

fn attachment_kind_label(kind: LinkGraphAttachmentKind) -> &'static str {
    match kind {
        LinkGraphAttachmentKind::Image => "image",
        LinkGraphAttachmentKind::Pdf => "pdf",
        LinkGraphAttachmentKind::Gpg => "gpg",
        LinkGraphAttachmentKind::Document => "document",
        LinkGraphAttachmentKind::Archive => "archive",
        LinkGraphAttachmentKind::Audio => "audio",
        LinkGraphAttachmentKind::Video => "video",
        LinkGraphAttachmentKind::Other => "other",
    }
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
    let language = super::support::source_language_label(Path::new(path.as_str()))
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

fn is_supported_code_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            CODE_CONTENT_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(ext))
        })
}

#[cfg(test)]
fn parse_content_search_line(line: &str) -> Option<(String, usize, String)> {
    let (path, remainder) = line.rsplit_once(':')?;
    let (path, line_number) = path.rsplit_once(':')?;
    Some((
        path.to_string(),
        line_number.parse().ok()?,
        remainder.to_string(),
    ))
}

fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn parse_repo_code_search_query(query: &str) -> ParsedRepoCodeSearchQuery {
    let mut spec = ParsedRepoCodeSearchQuery::default();
    let mut search_tokens = Vec::new();
    for token in query.split_whitespace() {
        if let Some(value) = token.strip_prefix("lang:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                spec.language_filters.insert(normalized);
            }
            continue;
        }

        if let Some(value) = token.strip_prefix("kind:") {
            let normalized = value.trim().to_ascii_lowercase();
            if matches!(
                normalized.as_str(),
                "file" | "symbol" | "function" | "module" | "example"
            ) {
                spec.kind_filters.insert(normalized);
                continue;
            }
        }

        search_tokens.push(token.to_string());
    }

    spec.search_term = (!search_tokens.is_empty()).then(|| search_tokens.join(" "));
    spec
}

fn path_matches_language_filters(path: &str, filters: &HashSet<String>) -> bool {
    if filters.is_empty() {
        return true;
    }

    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    filters.iter().any(|filter| match filter.as_str() {
        "julia" => matches!(extension.as_deref(), Some("jl" | "julia")),
        "modelica" => matches!(extension.as_deref(), Some("mo" | "modelica")),
        other => extension.as_deref() == Some(other),
    })
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

fn build_code_search_response(
    studio: &crate::gateway::studio::router::StudioState,
    raw_query: String,
    repo_hint: Option<String>,
    limit: usize,
) -> Result<SearchResponse, StudioApiError> {
    let parsed = parse_code_search_query(raw_query.as_str(), repo_hint.as_deref());
    let repositories = if let Some(repo_id) = parsed.repo.as_deref() {
        vec![configured_repository(studio, repo_id).map_err(map_repo_intelligence_error)?]
    } else {
        configured_repositories(studio)
    };

    if repositories.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for code search",
        ));
    }

    studio
        .repo_index
        .ensure_repositories_enqueued(repositories.clone(), false);

    let mut hits = Vec::new();
    let mut pending_repos = Vec::new();
    let mut skipped_repos = Vec::new();
    for repository in repositories {
        let Some(snapshot) = studio.repo_index.snapshot(repository.id.as_str()) else {
            let repo_status = studio
                .repo_index
                .status_response(Some(repository.id.as_str()));
            let phase = repo_status.repos.first().map(|status| status.phase);
            if matches!(
                phase,
                Some(RepoIndexPhase::Unsupported | RepoIndexPhase::Failed)
            ) {
                skipped_repos.push(repository.id.clone());
            } else {
                pending_repos.push(repository.id.clone());
            }
            continue;
        };
        let symbol_hits = build_symbol_search(
            &RepoSymbolSearchQuery {
                repo_id: repository.id.clone(),
                query: parsed.query.clone(),
                limit,
            },
            snapshot.analysis.as_ref(),
        )
        .symbol_hits;
        let module_hits = build_module_search(
            &ModuleSearchQuery {
                repo_id: repository.id.clone(),
                query: parsed.query.clone(),
                limit,
            },
            snapshot.analysis.as_ref(),
        )
        .module_hits;
        let example_hits = build_example_search(
            &ExampleSearchQuery {
                repo_id: repository.id.clone(),
                query: parsed.query.clone(),
                limit,
            },
            snapshot.analysis.as_ref(),
        )
        .example_hits;

        let mut repository_hits = Vec::new();
        repository_hits.extend(
            symbol_hits
                .into_iter()
                .map(|hit| symbol_search_hit_to_search_hit(&repository.id, hit))
                .filter(|hit| matches_code_filters(hit, &parsed)),
        );
        repository_hits.extend(
            module_hits
                .into_iter()
                .map(|hit| module_search_hit_to_search_hit(&repository.id, hit))
                .filter(|hit| matches_code_filters(hit, &parsed)),
        );
        repository_hits.extend(
            example_hits
                .into_iter()
                .map(|hit| example_search_hit_to_search_hit(&repository.id, hit))
                .filter(|hit| matches_code_filters(hit, &parsed)),
        );

        if repository_hits.is_empty() {
            repository_hits.extend(build_repo_content_search_hits(
                snapshot.as_ref(),
                raw_query.as_str(),
                limit,
            ));
        }

        hits.extend(repository_hits);
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.stem.cmp(&right.stem))
    });
    hits.truncate(limit);

    let hit_count = hits.len();
    let indexing_state = if pending_repos.is_empty() {
        "ready".to_string()
    } else if hit_count == 0 {
        "indexing".to_string()
    } else {
        "partial".to_string()
    };

    Ok(SearchResponse {
        query: raw_query,
        hit_count,
        hits,
        graph_confidence_score: None,
        selected_mode: Some("code_search".to_string()),
        intent: Some("code_search".to_string()),
        intent_confidence: Some(1.0),
        search_mode: Some("code_search".to_string()),
        partial: !pending_repos.is_empty() || !skipped_repos.is_empty(),
        indexing_state: Some(indexing_state),
        pending_repos,
        skipped_repos,
    })
}

fn build_repo_content_search_hits(
    snapshot: &RepoIndexSnapshot,
    raw_query: &str,
    limit: usize,
) -> Vec<SearchHit> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Vec::new();
    };
    if !parsed.kind_filters.is_empty() && !parsed.kind_filters.contains("file") {
        return Vec::new();
    }

    let needle = search_term.to_ascii_lowercase();
    let mut hits = Vec::new();

    for document in snapshot.code_documents.iter() {
        let relative_path = document.path.as_str();
        if is_excluded_code_content_path(relative_path) {
            continue;
        }
        if !is_supported_code_extension(relative_path) {
            continue;
        }
        if !path_matches_language_filters(relative_path, &parsed.language_filters) {
            continue;
        }

        let Some((line_number, snippet)) =
            first_matching_line(document.contents.as_ref(), needle.as_str())
        else {
            continue;
        };

        let mut tags = vec![
            snapshot.repo_id.clone(),
            "code".to_string(),
            "file".to_string(),
            "kind:file".to_string(),
        ];
        if let Some(language) = document
            .language
            .clone()
            .or_else(|| infer_code_language(relative_path))
        {
            tags.push(language.clone());
            tags.push(format!("lang:{language}"));
        }

        let stem = Path::new(relative_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(relative_path)
            .to_string();
        let hierarchy = Some(
            relative_path
                .split('/')
                .map(str::to_string)
                .collect::<Vec<_>>(),
        );

        hits.push(SearchHit {
            stem,
            title: Some(relative_path.to_string()),
            path: relative_path.to_string(),
            doc_type: Some("file".to_string()),
            tags,
            score: 0.72,
            best_section: Some(format!(
                "{line_number}: {}",
                truncate_content_search_snippet(snippet.as_str(), 140)
            )),
            match_reason: Some("repo_content_search".to_string()),
            hierarchical_uri: None,
            hierarchy,
            implicit_backlinks: None,
            implicit_backlink_items: None,
            audit_status: None,
            verification_state: None,
            saliency_score: None,
            navigation_target: Some(StudioNavigationTarget {
                path: format!("{}/{}", snapshot.repo_id, relative_path),
                category: "repo_code".to_string(),
                project_name: Some(snapshot.repo_id.clone()),
                root_label: Some(snapshot.repo_id.clone()),
                line: Some(line_number),
                line_end: Some(line_number),
                column: None,
            }),
        });

        if hits.len() >= limit {
            break;
        }
    }

    hits
}

fn knowledge_graph_hit_to_search_hit(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    hit: crate::link_graph::LinkGraphHit,
) -> SearchHit {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    let display_path = studio_display_path(
        project_root,
        config_root,
        projects,
        &metadata,
        hit.path.as_str(),
    );
    let hierarchy = Some(
        display_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );

    SearchHit {
        stem: hit.stem.clone(),
        title: (!hit.title.trim().is_empty()).then_some(hit.title.clone()),
        path: display_path,
        doc_type: hit.doc_type.clone(),
        tags: hit.tags.clone(),
        score: hit.score,
        best_section: hit.best_section.clone(),
        match_reason: hit
            .match_reason
            .clone()
            .or_else(|| Some("link_graph_search".to_string())),
        hierarchical_uri: None,
        hierarchy,
        saliency_score: None,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: None,
        implicit_backlink_items: None,
        navigation_target: Some(StudioNavigationTarget {
            path: hit.path,
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: None,
            line_end: None,
            column: None,
        }),
    }
}

fn studio_display_path(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    metadata: &super::project_scope::SearchProjectMetadata,
    path: &str,
) -> String {
    let normalized = path.replace('\\', "/");
    if projects.len() > 1
        && let Some(project_name) = metadata.project_name.as_deref()
    {
        let relative_to_project = projects
            .iter()
            .find(|project| project.name == project_name)
            .and_then(|project| {
                super::project_scope::resolve_project_root_path(config_root, project.root.as_str())
            })
            .and_then(|project_root_path| {
                let absolute_path = if Path::new(path).is_absolute() {
                    Path::new(path).to_path_buf()
                } else {
                    project_root.join(path)
                };
                absolute_path
                    .strip_prefix(project_root_path)
                    .ok()
                    .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            })
            .filter(|relative| !relative.is_empty())
            .unwrap_or_else(|| normalized.clone());

        if !relative_to_project.starts_with(&format!("{project_name}/")) {
            return format!("{project_name}/{relative_to_project}");
        }
    }

    normalized
}

fn is_excluded_code_content_path(path: &str) -> bool {
    CODE_CONTENT_EXCLUDE_GLOBS.iter().any(|pattern| {
        let prefix = pattern.trim_end_matches("/**");
        path == prefix || path.starts_with(&format!("{prefix}/"))
    })
}

fn first_matching_line(contents: &str, needle: &str) -> Option<(usize, String)> {
    contents
        .lines()
        .enumerate()
        .find_map(|(index, line)| {
            line.contains(needle)
                .then(|| (index + 1, line.trim().to_string()))
        })
        .or_else(|| {
            contents.lines().enumerate().find_map(|(index, line)| {
                line.to_ascii_lowercase()
                    .contains(needle)
                    .then(|| (index + 1, line.trim().to_string()))
            })
        })
}

fn parse_code_search_query(query: &str, repo_hint: Option<&str>) -> ParsedCodeSearchQuery {
    let mut parsed = ParsedCodeSearchQuery {
        repo: repo_hint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase),
        ..ParsedCodeSearchQuery::default()
    };
    let mut terms = Vec::new();

    for token in query.split_whitespace() {
        if let Some(value) = token.strip_prefix("lang:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() && !parsed.languages.contains(&normalized) {
                parsed.languages.push(normalized);
            }
            continue;
        }
        if let Some(value) = token.strip_prefix("kind:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() && !parsed.kinds.contains(&normalized) {
                parsed.kinds.push(normalized);
            }
            continue;
        }
        if let Some(value) = token.strip_prefix("repo:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                parsed.repo = Some(normalized);
            }
            continue;
        }
        terms.push(token);
    }

    parsed.query = terms.join(" ").trim().to_string();
    parsed
}

fn matches_code_filters(hit: &SearchHit, parsed: &ParsedCodeSearchQuery) -> bool {
    if parsed.query.is_empty() {
        return false;
    }

    let language = infer_code_language(hit.path.as_str());
    if !parsed.languages.is_empty()
        && !language
            .as_deref()
            .map(|value| parsed.languages.iter().any(|item| item == value))
            .unwrap_or(false)
    {
        return false;
    }

    if parsed.kinds.is_empty() {
        return true;
    }

    let doc_type = hit
        .doc_type
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let explicit_kind = hit
        .tags
        .iter()
        .find_map(|tag| tag.strip_prefix("kind:"))
        .map(str::to_ascii_lowercase);

    parsed.kinds.iter().any(|kind| {
        kind == &doc_type
            || explicit_kind
                .as_deref()
                .map(|value| value == kind)
                .unwrap_or(false)
    })
}

fn symbol_search_hit_to_search_hit(
    repo_id: &str,
    hit: crate::analyzers::SymbolSearchHit,
) -> SearchHit {
    let language = infer_code_language(hit.symbol.path.as_str());
    let kind = symbol_kind_tag(hit.symbol.kind);
    let mut tags = vec![
        repo_id.to_string(),
        "code".to_string(),
        "symbol".to_string(),
        format!("kind:{kind}"),
    ];
    if let Some(language) = language.as_deref() {
        tags.push(language.to_string());
        tags.push(format!("lang:{language}"));
    }
    if let Some(status) = hit.audit_status.clone() {
        tags.push(status);
    }

    SearchHit {
        stem: hit.symbol.name.clone(),
        title: Some(hit.symbol.qualified_name.clone()),
        path: hit.symbol.path.clone(),
        doc_type: Some("symbol".to_string()),
        tags,
        score: hit.saliency_score.or(hit.score).unwrap_or(0.0),
        best_section: hit
            .symbol
            .signature
            .clone()
            .or_else(|| Some(hit.symbol.qualified_name.clone())),
        match_reason: Some("repo_symbol_search".to_string()),
        hierarchical_uri: hit.hierarchical_uri,
        hierarchy: hit.hierarchy,
        saliency_score: hit.saliency_score,
        audit_status: hit.audit_status,
        verification_state: hit.verification_state,
        implicit_backlinks: hit.implicit_backlinks,
        implicit_backlink_items: map_backlink_items(hit.implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            repo_id,
            hit.symbol.path.as_str(),
            Some("repo_code".to_string()),
            hit.symbol.line_start,
            hit.symbol.line_end,
        )),
    }
}

fn module_search_hit_to_search_hit(
    repo_id: &str,
    hit: crate::analyzers::ModuleSearchHit,
) -> SearchHit {
    let language = infer_code_language(hit.module.path.as_str());
    let mut tags = vec![
        repo_id.to_string(),
        "code".to_string(),
        "module".to_string(),
        "kind:module".to_string(),
    ];
    if let Some(language) = language.as_deref() {
        tags.push(language.to_string());
        tags.push(format!("lang:{language}"));
    }

    SearchHit {
        stem: hit.module.qualified_name.clone(),
        title: Some(hit.module.qualified_name.clone()),
        path: hit.module.path.clone(),
        doc_type: Some("module".to_string()),
        tags,
        score: hit.saliency_score.or(hit.score).unwrap_or(0.0),
        best_section: Some(hit.module.module_id.clone()),
        match_reason: Some("repo_module_search".to_string()),
        hierarchical_uri: hit.hierarchical_uri,
        hierarchy: hit.hierarchy,
        saliency_score: hit.saliency_score,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: hit.implicit_backlinks,
        implicit_backlink_items: map_backlink_items(hit.implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            repo_id,
            hit.module.path.as_str(),
            Some("repo_code".to_string()),
            Some(1),
            None,
        )),
    }
}

fn example_search_hit_to_search_hit(
    repo_id: &str,
    hit: crate::analyzers::ExampleSearchHit,
) -> SearchHit {
    let language = infer_code_language(hit.example.path.as_str());
    let mut tags = vec![
        repo_id.to_string(),
        "code".to_string(),
        "example".to_string(),
        "kind:example".to_string(),
    ];
    if let Some(language) = language.as_deref() {
        tags.push(language.to_string());
        tags.push(format!("lang:{language}"));
    }

    SearchHit {
        stem: hit.example.title.clone(),
        title: Some(hit.example.title.clone()),
        path: hit.example.path.clone(),
        doc_type: Some("example".to_string()),
        tags,
        score: hit.saliency_score.or(hit.score).unwrap_or(0.0),
        best_section: hit.example.summary.clone(),
        match_reason: Some("repo_example_search".to_string()),
        hierarchical_uri: hit.hierarchical_uri,
        hierarchy: hit.hierarchy,
        saliency_score: hit.saliency_score,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: hit.implicit_backlinks,
        implicit_backlink_items: map_backlink_items(hit.implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            repo_id,
            hit.example.path.as_str(),
            Some("repo_code".to_string()),
            Some(1),
            None,
        )),
    }
}

fn map_backlink_items(
    items: Option<Vec<crate::analyzers::RepoBacklinkItem>>,
) -> Option<Vec<SearchBacklinkItem>> {
    items.map(|items| {
        items
            .into_iter()
            .map(|item| SearchBacklinkItem {
                id: item.id,
                title: item.title,
                path: item.path,
                kind: item.kind,
            })
            .collect()
    })
}

fn repo_navigation_target(
    repo_id: &str,
    path: &str,
    category: Option<String>,
    line: Option<usize>,
    line_end: Option<usize>,
) -> StudioNavigationTarget {
    let normalized_path = path.replace('\\', "/");
    let path = if normalized_path.starts_with(&format!("{repo_id}/")) {
        normalized_path
    } else {
        format!("{repo_id}/{normalized_path}")
    };
    StudioNavigationTarget {
        path,
        category: category.unwrap_or_else(|| "repo_code".to_string()),
        project_name: Some(repo_id.to_string()),
        root_label: Some(repo_id.to_string()),
        line,
        line_end,
        column: None,
    }
}

fn infer_code_language(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".jl") {
        return Some("julia".to_string());
    }
    if lower.ends_with(".mo") {
        return Some("modelica".to_string());
    }
    if lower.ends_with(".rs") {
        return Some("rust".to_string());
    }
    if lower.ends_with(".py") {
        return Some("python".to_string());
    }
    if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        return Some("typescript".to_string());
    }
    None
}

fn symbol_kind_tag(kind: crate::analyzers::RepoSymbolKind) -> &'static str {
    match kind {
        crate::analyzers::RepoSymbolKind::Function => "function",
        crate::analyzers::RepoSymbolKind::Type => "type",
        crate::analyzers::RepoSymbolKind::Constant => "constant",
        crate::analyzers::RepoSymbolKind::ModuleExport => "module_export",
        crate::analyzers::RepoSymbolKind::Other => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;

    #[test]
    fn parse_code_search_query_extracts_repo_lang_and_kind_filters() {
        let parsed = parse_code_search_query("repo:sciml lang:julia kind:function reexport", None);
        assert_eq!(parsed.query, "reexport");
        assert_eq!(parsed.repo.as_deref(), Some("sciml"));
        assert_eq!(parsed.languages, vec!["julia".to_string()]);
        assert_eq!(parsed.kinds, vec!["function".to_string()]);
    }

    #[test]
    fn search_query_deserializes_query_alias() {
        let query: SearchQuery = serde_json::from_value(serde_json::json!({
            "query": "reexport",
            "intent": "code_search",
        }))
        .expect("query alias should deserialize");

        assert_eq!(query.q.as_deref(), Some("reexport"));
        assert_eq!(query.intent.as_deref(), Some("code_search"));
    }

    #[test]
    fn symbol_search_hit_to_search_hit_preserves_backend_metadata() {
        let hit = symbol_search_hit_to_search_hit(
            "sciml",
            crate::analyzers::SymbolSearchHit {
                symbol: crate::analyzers::SymbolRecord {
                    repo_id: "sciml".to_string(),
                    symbol_id: "symbol:reexport".to_string(),
                    module_id: Some("module:BaseModelica".to_string()),
                    name: "reexport".to_string(),
                    qualified_name: "BaseModelica.reexport".to_string(),
                    kind: crate::analyzers::RepoSymbolKind::Function,
                    path: "src/BaseModelica.jl".to_string(),
                    line_start: Some(7),
                    line_end: Some(9),
                    signature: Some("reexport()".to_string()),
                    audit_status: Some("verified".to_string()),
                    verification_state: Some("verified".to_string()),
                    attributes: std::collections::BTreeMap::new(),
                },
                score: Some(0.8),
                rank: Some(1),
                saliency_score: Some(0.9),
                hierarchical_uri: Some("repo://sciml/symbol/reexport".to_string()),
                hierarchy: Some(vec!["src".to_string(), "BaseModelica.jl".to_string()]),
                implicit_backlinks: Some(vec!["doc:readme".to_string()]),
                implicit_backlink_items: Some(vec![crate::analyzers::RepoBacklinkItem {
                    id: "doc:readme".to_string(),
                    title: Some("README".to_string()),
                    path: Some("README.md".to_string()),
                    kind: Some("documents".to_string()),
                }]),
                projection_page_ids: Some(vec!["projection:1".to_string()]),
                audit_status: Some("verified".to_string()),
                verification_state: Some("verified".to_string()),
            },
        );

        assert_eq!(hit.doc_type.as_deref(), Some("symbol"));
        assert!(hit.tags.iter().any(|tag| tag == "lang:julia"));
        assert!(hit.tags.iter().any(|tag| tag == "kind:function"));
        assert_eq!(hit.score, 0.9);
        assert_eq!(
            hit.navigation_target.and_then(|target| target.project_name),
            Some("sciml".to_string())
        );
        assert_eq!(hit.audit_status.as_deref(), Some("verified"));
    }

    #[test]
    fn repo_content_search_hits_find_matching_julia_source_lines() {
        let snapshot = RepoIndexSnapshot {
            repo_id: "sciml".to_string(),
            analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
            code_documents: Arc::new(vec![crate::gateway::studio::repo_index::RepoCodeDocument {
                path: "src/BaseModelica.jl".to_string(),
                language: Some("julia".to_string()),
                contents: Arc::<str>::from(
                    "module BaseModelica\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
                ),
            }]),
        };

        let hits = build_repo_content_search_hits(&snapshot, "lang:julia reexport", 10);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].doc_type.as_deref(), Some("file"));
        assert_eq!(hits[0].path, "src/BaseModelica.jl");
        assert_eq!(hits[0].match_reason.as_deref(), Some("repo_content_search"));
        assert_eq!(
            hits[0]
                .navigation_target
                .as_ref()
                .and_then(|target| target.line),
            Some(3)
        );
    }

    #[test]
    fn build_code_search_response_skips_unsupported_repositories_when_searching_all_repos() {
        let temp = tempfile::tempdir().expect("tempdir");
        let valid_repo = temp.path().join("ValidPkg");
        fs::create_dir_all(valid_repo.join("src")).expect("create valid src");
        fs::write(
            valid_repo.join("Project.toml"),
            "name = \"ValidPkg\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
        )
        .expect("write project");
        fs::write(
            valid_repo.join("src").join("ValidPkg.jl"),
            "module ValidPkg\nusing ModelingToolkit\nend\n",
        )
        .expect("write valid source");

        let invalid_repo = temp.path().join("DiffEqApproxFun.jl");
        fs::create_dir_all(invalid_repo.join("src")).expect("create invalid src");
        fs::write(
            invalid_repo.join("src").join("DiffEqApproxFun.jl"),
            "module DiffEqApproxFun\nusing ApproxFun\nend\n",
        )
        .expect("write invalid source");

        let studio = crate::gateway::studio::router::StudioState::new_with_bootstrap_ui_config(
            Arc::new(crate::analyzers::bootstrap_builtin_registry().expect("bootstrap registry")),
        );
        studio.set_ui_config(crate::gateway::studio::types::UiConfig {
            projects: Vec::new(),
            repo_projects: vec![
                crate::gateway::studio::types::UiRepoProjectConfig {
                    id: "valid".to_string(),
                    root: Some(valid_repo.display().to_string()),
                    url: None,
                    git_ref: None,
                    refresh: None,
                    plugins: vec!["julia".to_string()],
                },
                crate::gateway::studio::types::UiRepoProjectConfig {
                    id: "invalid".to_string(),
                    root: Some(invalid_repo.display().to_string()),
                    url: None,
                    git_ref: None,
                    refresh: None,
                    plugins: vec!["julia".to_string()],
                },
            ],
        });
        studio
            .repo_index
            .set_snapshot_for_test(Arc::new(RepoIndexSnapshot {
                repo_id: "valid".to_string(),
                analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
                code_documents: Arc::new(vec![
                    crate::gateway::studio::repo_index::RepoCodeDocument {
                        path: "src/ValidPkg.jl".to_string(),
                        language: Some("julia".to_string()),
                        contents: Arc::<str>::from("module ValidPkg\nusing ModelingToolkit\nend\n"),
                    },
                ]),
            }));
        studio.repo_index.set_status_for_test(
            crate::gateway::studio::repo_index::RepoIndexEntryStatus {
                repo_id: "valid".to_string(),
                phase: crate::gateway::studio::repo_index::RepoIndexPhase::Ready,
                last_error: None,
                last_revision: Some("abc123".to_string()),
                updated_at: Some("2026-03-21T00:00:00Z".to_string()),
                attempt_count: 1,
            },
        );
        studio.repo_index.set_status_for_test(
            crate::gateway::studio::repo_index::RepoIndexEntryStatus {
                repo_id: "invalid".to_string(),
                phase: crate::gateway::studio::repo_index::RepoIndexPhase::Unsupported,
                last_error: Some("missing Project.toml".to_string()),
                last_revision: None,
                updated_at: Some("2026-03-21T00:00:00Z".to_string()),
                attempt_count: 1,
            },
        );

        let response = build_code_search_response(&studio, "ValidPkg".to_string(), None, 10)
            .expect("all-repo code search should skip unsupported repositories");

        assert_eq!(response.query, "ValidPkg");
        assert_eq!(response.selected_mode.as_deref(), Some("code_search"));
        assert!(response.partial);
        assert_eq!(response.skipped_repos, vec!["invalid".to_string()]);
        assert!(response.hits.iter().all(|hit| {
            hit.navigation_target
                .as_ref()
                .and_then(|target| target.project_name.as_deref())
                != Some("invalid")
        }));
    }
}
