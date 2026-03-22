use std::collections::HashSet;
use std::path::Path;

use crate::analyzers::{
    ExampleSearchQuery, ModuleSearchQuery, RepoBacklinkItem, RepoSymbolKind,
    SymbolSearchQuery as RepoSymbolSearchQuery, build_example_search, build_module_search,
    build_symbol_search,
};
use crate::gateway::studio::repo_index::{RepoIndexPhase, RepoIndexSnapshot};
use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::types::{
    SearchBacklinkItem, SearchHit, SearchResponse, StudioNavigationTarget,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct ParsedCodeSearchQuery {
    pub(super) query: String,
    pub(super) repo: Option<String>,
    pub(super) languages: Vec<String>,
    pub(super) kinds: Vec<String>,
}

pub(super) const CODE_CONTENT_EXTENSIONS: [&str; 4] = ["jl", "julia", "mo", "modelica"];
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
pub(super) fn build_code_search_response(
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

pub(super) fn build_repo_content_search_hits(
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

fn matches_code_filters(hit: &SearchHit, parsed: &ParsedCodeSearchQuery) -> bool {
    if parsed.query.is_empty() {
        return false;
    }

    let language = infer_code_language(hit.path.as_str());
    if !parsed.languages.is_empty()
        && !language
            .as_deref()
            .is_some_and(|value| parsed.languages.iter().any(|item| item == value))
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
        kind == &doc_type || explicit_kind.as_deref().is_some_and(|value| value == kind)
    })
}

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

fn path_has_extension(path: &str, expected: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(expected))
}

fn symbol_kind_tag(kind: RepoSymbolKind) -> &'static str {
    match kind {
        RepoSymbolKind::Function => "function",
        RepoSymbolKind::Type => "type",
        RepoSymbolKind::Constant => "constant",
        RepoSymbolKind::ModuleExport => "module_export",
        RepoSymbolKind::Other => "other",
    }
}

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
