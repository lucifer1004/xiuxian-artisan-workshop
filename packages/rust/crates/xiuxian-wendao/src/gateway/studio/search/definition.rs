//! Logic for resolving the best semantic definition for a query.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::gateway::studio::types::{AstSearchHit, DefinitionSearchHit, UiProjectConfig};
use crate::search::{FuzzyMatcher, FuzzySearchOptions, LexicalMatcher};

use super::project_scope::project_metadata_for_path;

/// Match mode for definition resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionMatchMode {
    /// Only allow exact name matches.
    ExactOnly,
    /// Prefer exact match, fall back to fuzzy if none found.
    ExactThenFuzzy,
}

/// Options for resolving a definition.
#[derive(Debug, Clone)]
pub struct DefinitionResolveOptions {
    /// Maximum number of candidates to consider.
    pub limit: usize,
    /// Match mode to use.
    pub match_mode: DefinitionMatchMode,
    /// Optional scope patterns to restrict resolution.
    pub scope_patterns: Option<Vec<String>>,
    /// Optional languages to restrict resolution.
    pub languages: Option<Vec<String>>,
    /// Optional source path used to prefer nearby definitions.
    pub preferred_source_path: Option<String>,
    /// Whether to include Markdown headings in resolution.
    pub include_markdown: bool,
    /// Shared fuzzy-search options used when fuzzy fallback is enabled.
    pub fuzzy_options: FuzzySearchOptions,
}

impl Default for DefinitionResolveOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            match_mode: DefinitionMatchMode::ExactThenFuzzy,
            scope_patterns: None,
            languages: None,
            preferred_source_path: None,
            include_markdown: true,
            fuzzy_options: FuzzySearchOptions::symbol_search(),
        }
    }
}

/// Resolves the best definition hit from a list of AST hits.
pub fn resolve_best_definition(
    query_str: &str,
    ast_hits: &[AstSearchHit],
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    options: &DefinitionResolveOptions,
) -> Option<DefinitionSearchHit> {
    resolve_definition_candidates(
        query_str,
        ast_hits,
        project_root,
        config_root,
        projects,
        options,
    )
    .into_iter()
    .next()
}

#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_definition_candidates(
    query_str: &str,
    ast_hits: &[AstSearchHit],
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    options: &DefinitionResolveOptions,
) -> Vec<DefinitionSearchHit> {
    let query = query_str.trim();
    if query.is_empty() {
        return Vec::new();
    }

    let scope_matcher = options
        .scope_patterns
        .as_ref()
        .and_then(|patterns| build_scope_matcher(patterns.as_slice()));
    let preferred_parent = options
        .preferred_source_path
        .as_deref()
        .map(Path::new)
        .and_then(Path::parent)
        .map(normalize_path);

    // 1. Filter by language/scope if needed
    let filtered_hits = ast_hits
        .iter()
        .filter(|hit| {
            if let Some(langs) = &options.languages
                && !langs.contains(&hit.language)
            {
                return false;
            }
            if !options.include_markdown && hit.language == "markdown" {
                return false;
            }
            if let Some(matcher) = &scope_matcher {
                let relative_path = normalize_match_path(project_root, hit.path.as_str());
                if !matcher.is_match(relative_path.as_str()) && !matcher.is_match(hit.path.as_str())
                {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect::<Vec<_>>();

    // 2. Try exact matches first
    let mut candidates = filtered_hits
        .iter()
        .filter(|hit| hit.name == query)
        .map(|hit| {
            (
                hit.clone(),
                definition_match_score(1.0_f64, hit.path.as_str(), preferred_parent.as_deref()),
            )
        })
        .collect::<Vec<_>>();

    // 3. Fall back to case-insensitive if needed
    if candidates.is_empty() {
        candidates = filtered_hits
            .iter()
            .filter(|hit| hit.name.eq_ignore_ascii_case(query))
            .map(|hit| {
                (
                    hit.clone(),
                    definition_match_score(
                        0.98_f64,
                        hit.path.as_str(),
                        preferred_parent.as_deref(),
                    ),
                )
            })
            .collect::<Vec<_>>();
    }

    // 4. Fall back to substring if enabled
    if candidates.is_empty() && matches!(options.match_mode, DefinitionMatchMode::ExactThenFuzzy) {
        candidates = filtered_hits
            .iter()
            .filter(|hit| hit.name.to_lowercase().contains(&query.to_lowercase()))
            .map(|hit| {
                (
                    hit.clone(),
                    definition_match_score(0.8_f64, hit.path.as_str(), preferred_parent.as_deref()),
                )
            })
            .collect::<Vec<_>>();
    }

    // 5. Fall back to lexical fuzzy if enabled
    if candidates.is_empty() && matches!(options.match_mode, DefinitionMatchMode::ExactThenFuzzy) {
        fn ast_hit_name(hit: &AstSearchHit) -> &str {
            hit.name.as_str()
        }

        let matcher = LexicalMatcher::new(
            filtered_hits.as_slice(),
            ast_hit_name,
            options.fuzzy_options,
        );
        let fuzzy_matches = matcher
            .search(query, options.limit)
            .unwrap_or_else(|error| panic!("lexical matcher is infallible: {error}"));
        candidates = fuzzy_matches
            .into_iter()
            .map(|fuzzy_match| {
                let path = fuzzy_match.item.path.clone();
                (
                    fuzzy_match.item,
                    definition_match_score(
                        f64::from(fuzzy_match.score),
                        path.as_str(),
                        preferred_parent.as_deref(),
                    ),
                )
            })
            .collect();
    }

    if candidates.is_empty() {
        return Vec::new();
    }

    // Sort candidates by some heuristic if there are multiple
    candidates.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_exact = a.0.name == query;
                let b_exact = b.0.name == query;
                b_exact.cmp(&a_exact)
            })
            .then_with(|| a.0.path.cmp(&b.0.path))
    });

    candidates
        .into_iter()
        .map(|(hit, score)| {
            definition_hit_from_ast(hit, score, project_root, config_root, projects)
        })
        .collect()
}

fn definition_hit_from_ast(
    hit: AstSearchHit,
    score: f64,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> DefinitionSearchHit {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    let mut navigation_target = hit.navigation_target;
    navigation_target
        .project_name
        .clone_from(&metadata.project_name);
    navigation_target
        .root_label
        .clone_from(&metadata.root_label);

    DefinitionSearchHit {
        name: hit.name,
        signature: hit.signature,
        path: hit.path,
        language: hit.language,
        crate_name: hit.crate_name,
        project_name: metadata.project_name,
        root_label: metadata.root_label,
        node_kind: hit.node_kind,
        owner_title: hit.owner_title,
        navigation_target,
        line_start: hit.line_start,
        line_end: hit.line_end,
        score,
        observation_hints: Vec::new(),
    }
}

fn build_scope_matcher(patterns: &[String]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let mut has_pattern = false;
    for pattern in patterns {
        let Ok(glob) = Glob::new(pattern) else {
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

fn normalize_match_path(project_root: &Path, path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        return path
            .strip_prefix(project_root)
            .map_or_else(|_| normalize_path(path), normalize_path);
    }

    normalize_path(path)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn definition_match_score(
    base_score: f64,
    candidate_path: &str,
    preferred_parent: Option<&str>,
) -> f64 {
    base_score + definition_scope_bonus(candidate_path, preferred_parent)
}

fn definition_scope_bonus(candidate_path: &str, preferred_parent: Option<&str>) -> f64 {
    let Some(preferred_parent) = preferred_parent else {
        return 0.0;
    };
    let candidate_parent = Path::new(candidate_path)
        .parent()
        .map(normalize_path)
        .unwrap_or_default();
    if candidate_parent == preferred_parent {
        0.15
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::gateway::studio::types::StudioNavigationTarget;

    fn ast_hit(name: &str) -> AstSearchHit {
        AstSearchHit {
            name: name.to_string(),
            signature: format!("fn {name}()"),
            path: "src/lib.rs".to_string(),
            language: "rust".to_string(),
            crate_name: "demo".to_string(),
            project_name: None,
            root_label: None,
            node_kind: Some("function".to_string()),
            owner_title: None,
            navigation_target: StudioNavigationTarget {
                path: "src/lib.rs".to_string(),
                category: "symbol".to_string(),
                project_name: None,
                root_label: None,
                line: Some(10),
                line_end: Some(12),
                column: Some(1),
            },
            line_start: 10,
            line_end: 12,
            score: 1.0,
        }
    }

    #[test]
    fn resolve_best_definition_uses_lexical_fallback_for_typos() {
        let hits = vec![ast_hit("spawn_local")];

        let result = resolve_best_definition(
            "spwan_local",
            hits.as_slice(),
            Path::new("."),
            Path::new("."),
            &[],
            &DefinitionResolveOptions::default(),
        )
        .unwrap_or_else(|| panic!("definition should resolve through fuzzy fallback"));

        assert_eq!(result.name, "spawn_local");
        assert!(result.score < 1.0);
        assert!(result.score > 0.0);
    }
}
