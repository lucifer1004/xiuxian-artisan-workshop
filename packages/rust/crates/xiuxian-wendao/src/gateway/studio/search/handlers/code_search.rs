use std::collections::HashSet;
#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use crate::analyzers::{RepoBacklinkItem, RepoSymbolKind};
use crate::gateway::studio::repo_index::RepoIndexPhase;
use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
#[cfg(test)]
use crate::gateway::studio::types::{SearchBacklinkItem, StudioNavigationTarget};
use crate::gateway::studio::types::{SearchHit, SearchResponse};
use crate::search_plane::{SearchCorpusKind, SearchPlaneCacheTtl};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct ParsedCodeSearchQuery {
    pub(super) query: String,
    pub(super) repo: Option<String>,
    pub(super) languages: Vec<String>,
    pub(super) kinds: Vec<String>,
}

#[cfg(test)]
pub(super) const CODE_CONTENT_EXTENSIONS: [&str; 4] = ["jl", "julia", "mo", "modelica"];
#[cfg(test)]
pub(super) const CODE_CONTENT_EXCLUDE_GLOBS: [&str; 7] = [
    ".git/**",
    ".cache/**",
    ".devenv/**",
    ".direnv/**",
    "node_modules/**",
    "target/**",
    "dist/**",
];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct ParsedRepoCodeSearchQuery {
    pub(super) language_filters: HashSet<String>,
    pub(super) kind_filters: HashSet<String>,
    pub(super) search_term: Option<String>,
}

impl ParsedRepoCodeSearchQuery {
    pub(super) fn search_term(&self) -> Option<&str> {
        self.search_term.as_deref()
    }
}

#[allow(clippy::too_many_lines)]
pub(super) async fn build_code_search_response(
    studio: &StudioState,
    raw_query: String,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<SearchResponse, StudioApiError> {
    let parsed = parse_code_search_query(raw_query.as_str(), repo_hint);
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
    let repo_ids = repositories
        .iter()
        .map(|repository| repository.id.clone())
        .collect::<Vec<_>>();
    let repo_status = studio.repo_index.status_response(parsed.repo.as_deref());
    let cache_key = studio
        .search_plane
        .repo_search_query_cache_key(
            "code_search",
            &[],
            &[
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ],
            &repo_status,
            repo_ids.as_slice(),
            raw_query.as_str(),
            limit,
            Some("code_search"),
            parsed.repo.as_deref(),
        )
        .await;
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let repo_phase_lookup = repo_status
        .repos
        .iter()
        .map(|status| (status.repo_id.as_str(), status.phase))
        .collect::<std::collections::HashMap<_, _>>();

    let mut hits = Vec::new();
    let mut pending_repos = Vec::new();
    let mut skipped_repos = Vec::new();
    for repository in repositories {
        let has_repo_entity_publication = studio
            .search_plane
            .has_published_repo_corpus(SearchCorpusKind::RepoEntity, repository.id.as_str())
            .await;
        let has_repo_content_publication = studio
            .search_plane
            .has_published_repo_corpus(SearchCorpusKind::RepoContentChunk, repository.id.as_str())
            .await;
        if !has_repo_entity_publication && !has_repo_content_publication {
            let phase = repo_phase_lookup.get(repository.id.as_str()).copied();
            if matches!(
                phase,
                Some(RepoIndexPhase::Unsupported | RepoIndexPhase::Failed)
            ) {
                skipped_repos.push(repository.id.clone());
            } else {
                pending_repos.push(repository.id.clone());
            }
            continue;
        }
        let mut repository_hits = if has_repo_entity_publication {
            build_repo_entity_search_hits(studio, repository.id.as_str(), raw_query.as_str(), limit)
                .await?
        } else {
            Vec::new()
        };

        if repository_hits.is_empty() && has_repo_content_publication {
            repository_hits.extend(
                build_repo_content_search_hits(
                    studio,
                    repository.id.as_str(),
                    raw_query.as_str(),
                    limit,
                )
                .await?,
            );
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

    let response = SearchResponse {
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
    };
    if let Some(cache_key) = cache_key.as_ref() {
        studio
            .search_plane
            .cache_set_json(cache_key, SearchPlaneCacheTtl::HotQuery, &response)
            .await;
    }
    Ok(response)
}

pub(super) async fn build_repo_entity_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    match studio
        .search_plane
        .search_repo_entities(
            repo_id,
            search_term,
            &parsed.language_filters,
            &parsed.kind_filters,
            limit,
        )
        .await
    {
        Ok(hits) => Ok(hits),
        Err(error) => Err(StudioApiError::internal(
            "REPO_ENTITY_SEARCH_FAILED",
            "Failed to query repo entity search plane",
            Some(error.to_string()),
        )),
    }
}

pub(super) async fn build_repo_content_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    if !parsed.kind_filters.is_empty() && !parsed.kind_filters.contains("file") {
        return Ok(Vec::new());
    }
    match studio
        .search_plane
        .search_repo_content_chunks(repo_id, search_term, &parsed.language_filters, limit)
        .await
    {
        Ok(hits) => Ok(hits),
        Err(error) => Err(StudioApiError::internal(
            "REPO_CONTENT_SEARCH_FAILED",
            "Failed to query repo content search plane",
            Some(error.to_string()),
        )),
    }
}

#[cfg(test)]
pub(super) fn parse_content_search_line(line: &str) -> Option<(String, usize, String)> {
    let (path, remainder) = line.rsplit_once(':')?;
    let (path, line_number) = path.rsplit_once(':')?;
    Some((
        path.to_string(),
        line_number.parse().ok()?,
        remainder.to_string(),
    ))
}

#[cfg(test)]
pub(super) fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(super) fn parse_repo_code_search_query(query: &str) -> ParsedRepoCodeSearchQuery {
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

#[cfg(test)]
pub(super) fn path_matches_language_filters(path: &str, filters: &HashSet<String>) -> bool {
    if filters.is_empty() {
        return true;
    }

    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    filters.iter().any(|filter| match filter.as_str() {
        "julia" => matches!(extension.as_deref(), Some("jl" | "julia")),
        "modelica" => matches!(extension.as_deref(), Some("mo" | "modelica")),
        other => extension.as_deref() == Some(other),
    })
}

pub(super) fn parse_code_search_query(
    query: &str,
    repo_hint: Option<&str>,
) -> ParsedCodeSearchQuery {
    let mut parsed = ParsedCodeSearchQuery {
        repo: repo_hint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
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
            let repo_id = value.trim();
            if !repo_id.is_empty() {
                parsed.repo = Some(repo_id.to_string());
            }
            continue;
        }
        terms.push(token);
    }

    parsed.query = terms.join(" ").trim().to_string();
    parsed
}

#[cfg(test)]
pub(super) fn symbol_search_hit_to_search_hit(
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

#[cfg(test)]
fn map_backlink_items(items: Option<Vec<RepoBacklinkItem>>) -> Option<Vec<SearchBacklinkItem>> {
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

#[cfg(test)]
pub(super) fn repo_navigation_target(
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

#[cfg(test)]
fn infer_code_language(path: &str) -> Option<String> {
    if path_has_extension(path, "jl") {
        return Some("julia".to_string());
    }
    if path_has_extension(path, "mo") {
        return Some("modelica".to_string());
    }
    if path_has_extension(path, "rs") {
        return Some("rust".to_string());
    }
    if path_has_extension(path, "py") {
        return Some("python".to_string());
    }
    if path_has_extension(path, "ts") || path_has_extension(path, "tsx") {
        return Some("typescript".to_string());
    }
    None
}

#[cfg(test)]
fn path_has_extension(path: &str, expected: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
fn symbol_kind_tag(kind: RepoSymbolKind) -> &'static str {
    match kind {
        RepoSymbolKind::Function => "function",
        RepoSymbolKind::Type => "type",
        RepoSymbolKind::Constant => "constant",
        RepoSymbolKind::ModuleExport => "module_export",
        RepoSymbolKind::Other => "other",
    }
}

#[cfg(test)]
pub(super) fn is_supported_code_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            CODE_CONTENT_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(ext))
        })
}
