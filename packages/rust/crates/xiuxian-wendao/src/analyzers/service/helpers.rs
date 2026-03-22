//! Helper functions for repository intelligence service operations.

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::projection::build_projected_pages;
use crate::analyzers::query::RepoBacklinkItem;
use crate::analyzers::records::{
    DocRecord, ImportRecord, ModuleRecord, RelationKind, RelationRecord, SymbolRecord,
};

/// Returns a human-readable label for a relation kind.
#[must_use]
pub fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Contains => "contains",
        RelationKind::Calls => "calls",
        RelationKind::Uses => "uses",
        RelationKind::Documents => "documents",
        RelationKind::ExampleOf => "example_of",
        RelationKind::Declares => "declares",
        RelationKind::Implements => "implements",
        RelationKind::Imports => "imports",
    }
}

pub(crate) fn repo_hierarchical_uri(repo_id: &str) -> String {
    format!("repo://{repo_id}")
}

pub(crate) fn record_hierarchical_uri(
    repo_id: &str,
    ecosystem: &str,
    scope: &str,
    module_path: &str,
    record_id: &str,
) -> String {
    let clean_module = module_path.trim_matches('/').replace('/', ":");
    format!("wendao://repo/{ecosystem}/{repo_id}/{scope}/{clean_module}/{record_id}")
}

pub(crate) fn infer_ecosystem(repo_id: &str) -> &'static str {
    let lower = repo_id.to_ascii_lowercase();
    if lower.contains("sciml") || lower.contains("diffeq") {
        "sciml"
    } else if lower.contains("modelica") || lower == "msl" {
        "msl"
    } else {
        "unknown"
    }
}

pub(crate) fn hierarchy_segments_from_path(path: &str) -> Option<Vec<String>> {
    let segments = path
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!segments.is_empty()).then_some(segments)
}

pub(crate) fn normalized_rank_score(raw_score: u8, worst_bucket: u8) -> f64 {
    let denominator = f64::from(worst_bucket.saturating_add(1));
    let numerator = f64::from(worst_bucket.saturating_add(1).saturating_sub(raw_score));
    (numerator / denominator).clamp(0.0, 1.0)
}

pub(crate) fn documents_backlink_lookup(
    relations: &[RelationRecord],
    docs: &[DocRecord],
) -> BTreeMap<String, Vec<RepoBacklinkItem>> {
    let doc_lookup = docs
        .iter()
        .map(|doc| (doc.doc_id.as_str(), doc))
        .collect::<BTreeMap<_, _>>();
    let mut lookup: BTreeMap<String, BTreeMap<String, RepoBacklinkItem>> = BTreeMap::new();

    for relation in relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::Documents)
    {
        let source_id = relation.source_id.trim();
        let target_id = relation.target_id.trim();
        if source_id.is_empty() || target_id.is_empty() {
            continue;
        }
        let item = doc_lookup.get(source_id).map_or_else(
            || RepoBacklinkItem {
                id: source_id.to_string(),
                title: None,
                path: None,
                kind: Some("documents".to_string()),
            },
            |doc| RepoBacklinkItem {
                id: doc.doc_id.clone(),
                title: Some(doc.title.clone()).filter(|title| !title.trim().is_empty()),
                path: Some(doc.path.clone()).filter(|path| !path.trim().is_empty()),
                kind: Some("documents".to_string()),
            },
        );
        lookup
            .entry(target_id.to_string())
            .or_default()
            .insert(item.id.clone(), item);
    }

    lookup
        .into_iter()
        .map(|(target_id, sources)| (target_id, sources.into_values().collect::<Vec<_>>()))
        .collect()
}

pub(crate) fn backlinks_for(
    target_id: &str,
    lookup: &BTreeMap<String, Vec<RepoBacklinkItem>>,
) -> (Option<Vec<String>>, Option<Vec<RepoBacklinkItem>>) {
    let Some(backlinks) = lookup.get(target_id) else {
        return (None, None);
    };
    let items = backlinks
        .iter()
        .filter_map(|backlink| {
            let id = backlink.id.trim();
            (!id.is_empty()).then(|| RepoBacklinkItem {
                id: id.to_string(),
                title: backlink
                    .title
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                path: backlink
                    .path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                kind: backlink
                    .kind
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
            })
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        return (None, None);
    }
    let ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    (Some(ids), Some(items))
}

pub(crate) fn projection_page_lookup(
    analysis: &RepositoryAnalysisOutput,
) -> BTreeMap<String, Vec<String>> {
    let mut lookup: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for page in build_projected_pages(analysis) {
        for anchor in page
            .module_ids
            .iter()
            .chain(page.symbol_ids.iter())
            .chain(page.example_ids.iter())
            .chain(page.doc_ids.iter())
        {
            lookup
                .entry(anchor.clone())
                .or_default()
                .insert(page.page_id.clone());
        }
    }

    lookup
        .into_iter()
        .map(|(anchor, page_ids)| (anchor, page_ids.into_iter().collect::<Vec<_>>()))
        .collect()
}

pub(crate) fn projection_pages_for(
    anchor_id: &str,
    lookup: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    lookup.get(anchor_id).and_then(|page_ids| {
        let filtered = page_ids
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        (!filtered.is_empty()).then_some(filtered)
    })
}

pub(crate) fn module_match_score(query: &str, qualified_name: &str, path: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    if qualified_name == query {
        return Some(0);
    }
    if qualified_name.starts_with(query) {
        return Some(1);
    }
    if qualified_name.contains(query) {
        return Some(2);
    }
    if path.contains(query) {
        return Some(3);
    }
    None
}

pub(crate) fn symbol_match_score(
    query: &str,
    name: &str,
    qualified_name: &str,
    path: &str,
    signature: &str,
) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    if name == query {
        return Some(0);
    }
    if qualified_name == query {
        return Some(1);
    }
    if name.starts_with(query) {
        return Some(2);
    }
    if qualified_name.starts_with(query) {
        return Some(3);
    }
    if name.contains(query) {
        return Some(4);
    }
    if qualified_name.contains(query) {
        return Some(5);
    }
    if signature.contains(query) {
        return Some(6);
    }
    if path.contains(query) {
        return Some(7);
    }
    None
}

pub(crate) fn example_match_score(
    query: &str,
    title: &str,
    path: &str,
    summary: &str,
    related_symbols: &[String],
    related_modules: &[String],
) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    if title == query {
        return Some(0);
    }
    if title.starts_with(query) {
        return Some(1);
    }
    if title.contains(query) {
        return Some(2);
    }
    if related_symbols.iter().any(|candidate| candidate == query) {
        return Some(3);
    }
    if related_modules.iter().any(|candidate| candidate == query) {
        return Some(4);
    }
    if related_symbols
        .iter()
        .any(|candidate| candidate.starts_with(query))
    {
        return Some(5);
    }
    if related_modules
        .iter()
        .any(|candidate| candidate.starts_with(query))
    {
        return Some(6);
    }
    if path.contains(query) {
        return Some(7);
    }
    if summary.contains(query) {
        return Some(8);
    }
    if related_symbols
        .iter()
        .any(|candidate| candidate.contains(query))
    {
        return Some(9);
    }
    if related_modules
        .iter()
        .any(|candidate| candidate.contains(query))
    {
        return Some(10);
    }
    None
}

pub(crate) fn import_match_score(
    package_filter: Option<&str>,
    module_filter: Option<&str>,
    import: &ImportRecord,
) -> Option<u8> {
    if package_filter.is_none() && module_filter.is_none() {
        return Some(0);
    }

    let target_lower = import.target_package.to_ascii_lowercase();
    let source_lower = import.source_module.to_ascii_lowercase();

    if let Some(package_filter) = package_filter {
        if package_filter == target_lower {
            if let Some(module_filter) = module_filter {
                if module_filter == source_lower {
                    return Some(0);
                }
                if source_lower.starts_with(module_filter) {
                    return Some(1);
                }
                return None;
            }
            return Some(0);
        }
        if target_lower.starts_with(package_filter) {
            if let Some(module_filter) = module_filter {
                if module_filter == source_lower || source_lower.starts_with(module_filter) {
                    return Some(2);
                }
                return None;
            }
            return Some(1);
        }
        if target_lower.contains(package_filter) {
            return Some(3);
        }
        return None;
    }

    if let Some(module_filter) = module_filter {
        if module_filter == source_lower {
            return Some(0);
        }
        if source_lower.starts_with(module_filter) {
            return Some(1);
        }
        if source_lower.contains(module_filter) {
            return Some(2);
        }
        return None;
    }

    Some(0)
}

pub(crate) fn example_relation_lookup(relations: &[RelationRecord]) -> BTreeSet<(String, String)> {
    relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::ExampleOf)
        .map(|relation| (relation.source_id.clone(), relation.target_id.clone()))
        .collect()
}

pub(crate) fn related_symbols_for_example(
    example_id: &str,
    relation_lookup: &BTreeSet<(String, String)>,
    symbols: &[SymbolRecord],
) -> Vec<String> {
    let symbol_ids = relation_lookup
        .iter()
        .filter(|(source_id, _)| source_id == example_id)
        .map(|(_, target_id)| target_id.as_str())
        .collect::<BTreeSet<_>>();

    symbols
        .iter()
        .filter(|symbol| symbol_ids.contains(symbol.symbol_id.as_str()))
        .flat_map(|symbol| {
            [
                symbol.name.to_ascii_lowercase(),
                symbol.qualified_name.to_ascii_lowercase(),
            ]
        })
        .collect()
}

pub(crate) fn related_modules_for_example(
    example_id: &str,
    relation_lookup: &BTreeSet<(String, String)>,
    modules: &[ModuleRecord],
) -> Vec<String> {
    let module_ids = relation_lookup
        .iter()
        .filter(|(source_id, _)| source_id == example_id)
        .map(|(_, target_id)| target_id.as_str())
        .collect::<BTreeSet<_>>();

    modules
        .iter()
        .filter(|module| module_ids.contains(module.module_id.as_str()))
        .flat_map(|module| {
            let short_name = module
                .qualified_name
                .rsplit('.')
                .next()
                .unwrap_or(module.qualified_name.as_str())
                .to_ascii_lowercase();
            [module.qualified_name.to_ascii_lowercase(), short_name]
        })
        .collect()
}

pub(crate) fn resolve_module_scope<'a>(
    module_selector: Option<&str>,
    modules: &'a [ModuleRecord],
) -> Option<&'a ModuleRecord> {
    let selector = module_selector?.trim();
    if selector.is_empty() {
        return None;
    }

    modules.iter().find(|module| {
        module.module_id == selector || module.qualified_name == selector || module.path == selector
    })
}

pub(crate) fn docs_in_scope(
    scoped_module: Option<&ModuleRecord>,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<DocRecord> {
    match scoped_module {
        None => analysis.docs.clone(),
        Some(module) => {
            let mut target_ids = BTreeSet::from([module.module_id.clone()]);
            target_ids.extend(
                symbols_in_scope(Some(module), &analysis.symbols)
                    .into_iter()
                    .map(|symbol| symbol.symbol_id.clone()),
            );
            let doc_ids = analysis
                .relations
                .iter()
                .filter(|relation| {
                    relation.kind == RelationKind::Documents
                        && target_ids.contains(relation.target_id.as_str())
                })
                .map(|relation| relation.source_id.clone())
                .collect::<BTreeSet<_>>();
            analysis
                .docs
                .iter()
                .filter(|doc| doc_ids.contains(doc.doc_id.as_str()))
                .cloned()
                .collect()
        }
    }
}

pub(crate) fn documented_symbol_ids(
    scoped_module: Option<&ModuleRecord>,
    symbols: &[SymbolRecord],
    relations: &[RelationRecord],
) -> BTreeSet<String> {
    let scoped_symbol_ids = symbols_in_scope(scoped_module, symbols)
        .into_iter()
        .map(|symbol| symbol.symbol_id.clone())
        .collect::<BTreeSet<_>>();

    relations
        .iter()
        .filter(|relation| {
            relation.kind == RelationKind::Documents
                && scoped_symbol_ids.contains(&relation.target_id)
        })
        .map(|relation| relation.target_id.clone())
        .collect()
}

pub(crate) fn symbols_in_scope<'a>(
    scoped_module: Option<&ModuleRecord>,
    symbols: &'a [SymbolRecord],
) -> Vec<&'a SymbolRecord> {
    match scoped_module {
        None => symbols.iter().collect(),
        Some(module) => symbols
            .iter()
            .filter(|symbol| symbol.module_id.as_deref() == Some(module.module_id.as_str()))
            .collect(),
    }
}
