use std::collections::{BTreeMap, HashSet, VecDeque};
#[cfg(test)]
use std::path::Path;

use tokio::task::JoinSet;

#[cfg(test)]
use crate::analyzers::{RepoBacklinkItem, RepoSymbolKind};
use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
#[cfg(test)]
use crate::gateway::studio::types::{SearchBacklinkItem, StudioNavigationTarget};
use crate::gateway::studio::types::{SearchHit, SearchResponse};
use crate::search_plane::{
    RepoSearchAvailability, RepoSearchPublicationState, RepoSearchQueryCacheKeyInput,
    SearchCorpusKind, SearchPlaneCacheTtl, SearchPlaneService,
};

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
pub(crate) const CODE_CONTENT_EXCLUDE_GLOBS: [&str; 7] = [
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

#[derive(Debug, Clone)]
pub(super) struct RepoSearchTarget {
    pub(super) repo_id: String,
    pub(super) publication_state: RepoSearchPublicationState,
}

#[derive(Debug, Default)]
pub(super) struct RepoSearchDispatch {
    pub(super) searchable_repos: Vec<RepoSearchTarget>,
    pub(super) pending_repos: Vec<String>,
    pub(super) skipped_repos: Vec<String>,
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
    let repo_ids = if let Some(repo_id) = parsed.repo.as_deref() {
        vec![
            configured_repository(studio, repo_id)
                .map_err(map_repo_intelligence_error)?
                .id,
        ]
    } else {
        configured_repositories(studio)
            .into_iter()
            .map(|repository| repository.id)
            .collect::<Vec<_>>()
    };

    if repo_ids.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for code search",
        ));
    }
    let cache_key = studio
        .search_plane
        .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
            scope: "code_search",
            corpora: &[],
            repo_corpora: &[
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ],
            repo_ids: repo_ids.as_slice(),
            query: raw_query.as_str(),
            limit,
            intent: Some("code_search"),
            repo_hint: parsed.repo.as_deref(),
        })
        .await;
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let mut hits = Vec::new();
    let publication_states = studio
        .search_plane
        .repo_search_publication_states(repo_ids.as_slice())
        .await;
    let dispatch = collect_repo_search_targets(repo_ids, &publication_states);
    let pending_repos = dispatch.pending_repos;
    let skipped_repos = dispatch.skipped_repos;
    hits.extend(
        search_repo_code_hits_buffered(
            studio.search_plane.clone(),
            dispatch.searchable_repos,
            raw_query.as_str(),
            limit,
        )
        .await?,
    );

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

pub(super) async fn search_repo_entity_hits(
    search_plane: &SearchPlaneService,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    match search_plane
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

pub(super) async fn search_repo_content_hits(
    search_plane: &SearchPlaneService,
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
    match search_plane
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
pub(super) async fn build_repo_entity_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_entity_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

#[cfg(test)]
pub(super) async fn build_repo_content_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_content_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

pub(super) fn collect_repo_search_targets(
    repo_ids: Vec<String>,
    publication_states: &BTreeMap<String, RepoSearchPublicationState>,
) -> RepoSearchDispatch {
    let mut dispatch = RepoSearchDispatch::default();
    for repo_id in repo_ids {
        let publication_state = publication_states.get(repo_id.as_str()).copied().unwrap_or(
            RepoSearchPublicationState {
                entity_published: false,
                content_published: false,
                availability: RepoSearchAvailability::Pending,
            },
        );
        if publication_state.is_searchable() {
            dispatch.searchable_repos.push(RepoSearchTarget {
                repo_id,
                publication_state,
            });
            continue;
        }
        match publication_state.availability {
            RepoSearchAvailability::Skipped => dispatch.skipped_repos.push(repo_id),
            RepoSearchAvailability::Pending => dispatch.pending_repos.push(repo_id),
            RepoSearchAvailability::Searchable => {}
        }
    }
    dispatch
}

pub(super) fn repo_search_parallelism(repo_count: usize) -> usize {
    if repo_count == 0 {
        return 1;
    }
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(4)
        .max(1)
        .min(repo_count)
}

async fn search_repo_code_hits_buffered(
    search_plane: SearchPlaneService,
    targets: Vec<RepoSearchTarget>,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }

    let mut queued = VecDeque::from(targets);
    let mut join_set = JoinSet::new();
    let raw_query = raw_query.to_string();
    let parallelism = repo_search_parallelism(queued.len());
    for _ in 0..parallelism {
        if let Some(target) = queued.pop_front() {
            spawn_repo_code_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                limit,
            );
        }
    }

    let mut hits = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let repository_hits = result.map_err(|error| {
            StudioApiError::internal(
                "REPO_CODE_SEARCH_TASK_FAILED",
                "Repo code-search task failed",
                Some(error.to_string()),
            )
        })??;
        hits.extend(repository_hits);
        if let Some(target) = queued.pop_front() {
            spawn_repo_code_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                limit,
            );
        }
    }
    Ok(hits)
}

fn spawn_repo_code_search_task(
    join_set: &mut JoinSet<Result<Vec<SearchHit>, StudioApiError>>,
    search_plane: SearchPlaneService,
    target: RepoSearchTarget,
    raw_query: String,
    limit: usize,
) {
    join_set.spawn(async move {
        let mut repository_hits = if target.publication_state.entity_published {
            search_repo_entity_hits(
                &search_plane,
                target.repo_id.as_str(),
                raw_query.as_str(),
                limit,
            )
            .await?
        } else {
            Vec::new()
        };

        if repository_hits.is_empty() && target.publication_state.content_published {
            repository_hits.extend(
                search_repo_content_hits(
                    &search_plane,
                    target.repo_id.as_str(),
                    raw_query.as_str(),
                    limit,
                )
                .await?,
            );
        }

        Ok(repository_hits)
    });
}

#[cfg(test)]
pub(crate) fn parse_content_search_line(line: &str) -> Option<(String, usize, String)> {
    let (path, remainder) = line.rsplit_once(':')?;
    let (path, line_number) = path.rsplit_once(':')?;
    Some((
        path.to_string(),
        line_number.parse().ok()?,
        remainder.to_string(),
    ))
}

#[cfg(test)]
pub(crate) fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(crate) fn parse_repo_code_search_query(query: &str) -> ParsedRepoCodeSearchQuery {
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
pub(crate) fn path_matches_language_filters(path: &str, filters: &HashSet<String>) -> bool {
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
pub(crate) fn repo_navigation_target(
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
pub(crate) fn is_supported_code_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            CODE_CONTENT_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(ext))
        })
}
