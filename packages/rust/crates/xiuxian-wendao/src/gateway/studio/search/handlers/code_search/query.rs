#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use super::types::ParsedCodeSearchQuery;
#[cfg(test)]
pub(crate) use crate::search::repo_search::RepoSearchResultLimits;

#[cfg(test)]
const DEFAULT_REPO_WIDE_CODE_SEARCH_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const DEFAULT_REPO_WIDE_PER_REPO_ENTITY_RESULT_LIMIT: usize = 12;
#[cfg(test)]
const DEFAULT_REPO_WIDE_PER_REPO_CONTENT_RESULT_LIMIT: usize = 4;

#[cfg(test)]
pub(crate) fn repo_wide_code_search_timeout(repo_hint: Option<&str>) -> Option<Duration> {
    repo_hint
        .is_none()
        .then_some(DEFAULT_REPO_WIDE_CODE_SEARCH_TIMEOUT)
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn parse_code_search_query(
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
pub(crate) fn infer_repo_hint_from_query<'a, I>(
    parsed: &ParsedCodeSearchQuery,
    repo_ids: I,
) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    if parsed.repo.is_some() {
        return None;
    }

    let normalized_query = normalize_repo_search_seed(parsed.query.as_str());
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

#[cfg(test)]
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
