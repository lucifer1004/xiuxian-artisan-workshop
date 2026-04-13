use std::collections::HashSet;
use std::time::Duration;

use crate::analyzers::RegisteredRepository;
use crate::parsers::search::repo_code_query::ParsedRepoCodeSearchQuery;
pub(crate) use crate::search::repo_search::RepoSearchResultLimits;

const DEFAULT_REPO_WIDE_CODE_SEARCH_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_REPO_WIDE_PER_REPO_ENTITY_RESULT_LIMIT: usize = 12;
const DEFAULT_REPO_WIDE_PER_REPO_CONTENT_RESULT_LIMIT: usize = 4;

pub(crate) fn repo_wide_code_search_timeout(repo_hint: Option<&str>) -> Option<Duration> {
    repo_hint
        .is_none()
        .then_some(DEFAULT_REPO_WIDE_CODE_SEARCH_TIMEOUT)
}

pub(crate) fn repo_search_result_limits(
    repo_hint: Option<&str>,
    limit: usize,
) -> RepoSearchResultLimits {
    if repo_hint.is_some() {
        return RepoSearchResultLimits {
            entity_limit: limit,
            content_limit: limit,
        };
    }

    RepoSearchResultLimits {
        entity_limit: limit.min(DEFAULT_REPO_WIDE_PER_REPO_ENTITY_RESULT_LIMIT),
        content_limit: limit.min(DEFAULT_REPO_WIDE_PER_REPO_CONTENT_RESULT_LIMIT),
    }
}

pub(crate) fn infer_repo_hint_from_query<'a, I>(
    parsed: &ParsedRepoCodeSearchQuery,
    repo_ids: I,
) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    if parsed.repo.is_some() {
        return None;
    }

    let normalized_query = normalize_repo_search_seed(parsed.search_term().unwrap_or_default());
    if normalized_query.is_empty() {
        return None;
    }

    let mut matches = repo_ids
        .into_iter()
        .filter(|repo_id| normalize_repo_search_seed(repo_id) == normalized_query);
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some(first.to_string())
}

pub(crate) fn infer_repo_hint_from_repositories(
    parsed: &ParsedRepoCodeSearchQuery,
    repositories: &[RegisteredRepository],
) -> Option<String> {
    if let Some(repo_id) = infer_repo_hint_from_query(
        parsed,
        repositories.iter().map(|repository| repository.id.as_str()),
    ) {
        return Some(repo_id);
    }

    let normalized_query = normalize_repo_search_seed(parsed.search_term().unwrap_or_default());
    if normalized_query.is_empty() {
        return None;
    }
    let mut matches = repositories.iter().filter(|repository| {
        repository_search_seeds(repository).contains(normalized_query.as_str())
    });
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some(first.id.clone())
}

pub(crate) fn query_uses_redundant_repo_seed(
    parsed: &ParsedRepoCodeSearchQuery,
    repository: &RegisteredRepository,
) -> bool {
    if parsed.ast_pattern.is_none() && parsed.language_filters.is_empty() {
        return false;
    }

    let Some(search_term) = parsed.search_term() else {
        return false;
    };
    let normalized_query = normalize_repo_search_seed(search_term);
    if normalized_query.is_empty() {
        return false;
    }

    repository_search_seeds(repository).contains(normalized_query.as_str())
}

fn normalize_repo_search_seed(value: &str) -> String {
    let mut normalized = value.trim().to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_suffix(".jl") {
        normalized = stripped.to_string();
    }

    let mut collapsed = String::with_capacity(normalized.len());
    let mut in_whitespace = true;
    for character in normalized.chars() {
        let mapped = if matches!(character, '_' | '.' | '/' | '-') {
            ' '
        } else {
            character
        };
        if mapped.is_ascii_whitespace() {
            if !in_whitespace {
                collapsed.push(' ');
            }
            in_whitespace = true;
        } else {
            collapsed.push(mapped);
            in_whitespace = false;
        }
    }

    collapsed.trim().to_string()
}

fn repository_search_seeds(repository: &RegisteredRepository) -> HashSet<String> {
    let mut seeds = HashSet::new();
    push_repo_search_seed(&mut seeds, repository.id.as_str());

    if let Some(path) = repository.path.as_deref()
        && let Some(file_name) = path.file_name().and_then(|value| value.to_str())
    {
        push_repo_search_seed(&mut seeds, file_name);
    }

    if let Some(url) = repository.url.as_deref() {
        push_repo_search_seed(&mut seeds, url);
        if let Some(last_segment) = repository_url_last_segment(url) {
            push_repo_search_seed(&mut seeds, last_segment);
        }
    }

    seeds
}

fn push_repo_search_seed(seeds: &mut HashSet<String>, value: &str) {
    let normalized = normalize_repo_search_seed(value);
    if !normalized.is_empty() {
        seeds.insert(normalized);
    }
}

fn repository_url_last_segment(url: &str) -> Option<&str> {
    let trimmed = url.trim().trim_end_matches('/');
    let last_segment = trimmed.rsplit('/').next()?;
    Some(last_segment.strip_suffix(".git").unwrap_or(last_segment))
}
