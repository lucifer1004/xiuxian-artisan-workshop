//! Repository search functions (overview, module, symbol, example, import, doc coverage).

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::query::{
    DocCoverageQuery, DocCoverageResult, ExampleSearchHit, ExampleSearchQuery, ExampleSearchResult,
    ImportSearchHit, ImportSearchQuery, ImportSearchResult, ModuleSearchHit, ModuleSearchQuery,
    ModuleSearchResult, RepoOverviewQuery, RepoOverviewResult, SymbolSearchHit, SymbolSearchQuery,
    SymbolSearchResult,
};
use crate::analyzers::records::{ExampleRecord, ImportRecord, ModuleRecord, SymbolRecord};
use crate::analyzers::registry::PluginRegistry;
use crate::search::{FuzzySearchOptions, SearchDocument, SearchDocumentIndex};

use super::helpers::{
    backlinks_for, docs_in_scope, documented_symbol_ids, documents_backlink_lookup,
    example_match_score, example_relation_lookup, hierarchy_segments_from_path, import_match_score,
    infer_ecosystem, module_match_score, normalized_rank_score, projection_page_lookup,
    projection_pages_for, record_hierarchical_uri, related_modules_for_example,
    related_symbols_for_example, repo_hierarchical_uri, resolve_module_scope, symbol_match_score,
    symbols_in_scope,
};
use super::{analyze_repository_from_config_with_registry, bootstrap_builtin_registry};

const MODULE_SEARCH_BUCKETS: u8 = 3;
const SYMBOL_SEARCH_BUCKETS: u8 = 7;
const EXAMPLE_SEARCH_BUCKETS: u8 = 10;
const SEARCH_CANDIDATE_MULTIPLIER: usize = 8;

#[derive(Debug, Clone)]
struct RankedSearchRecord<T> {
    item: T,
    score: f64,
}

#[derive(Debug, Clone, Default)]
struct ExampleSearchMetadata {
    related_symbols: Vec<String>,
    related_modules: Vec<String>,
}

fn search_candidate_limit(limit: usize) -> usize {
    limit.max(1).saturating_mul(SEARCH_CANDIDATE_MULTIPLIER)
}

fn build_search_document_index<I>(documents: I) -> Option<SearchDocumentIndex>
where
    I: IntoIterator<Item = SearchDocument>,
{
    let index = SearchDocumentIndex::new();
    index.add_documents(documents).ok()?;
    Some(index)
}

fn module_search_document(module: &ModuleRecord) -> SearchDocument {
    let namespace = module
        .qualified_name
        .rsplit_once('.')
        .map(|(namespace, _name)| namespace.to_string())
        .unwrap_or_default();

    SearchDocument {
        id: module.module_id.clone(),
        title: module.qualified_name.clone(),
        kind: "module".to_string(),
        path: module.path.clone(),
        scope: module.repo_id.clone(),
        namespace,
        terms: vec![module.qualified_name.clone(), module.path.clone()],
    }
}

fn symbol_search_document(symbol: &SymbolRecord) -> SearchDocument {
    let mut terms = vec![
        symbol.name.clone(),
        symbol.qualified_name.clone(),
        symbol.path.clone(),
    ];
    if let Some(signature) = &symbol.signature {
        terms.push(signature.clone());
    }
    if let Some(module_id) = &symbol.module_id {
        terms.push(module_id.clone());
    }
    terms.extend(symbol.attributes.values().cloned());

    SearchDocument {
        id: symbol.symbol_id.clone(),
        title: symbol.qualified_name.clone(),
        kind: format!("{:?}", symbol.kind).to_ascii_lowercase(),
        path: symbol.path.clone(),
        scope: symbol.repo_id.clone(),
        namespace: symbol.module_id.clone().unwrap_or_default(),
        terms,
    }
}

fn example_search_document(
    example: &ExampleRecord,
    metadata: &ExampleSearchMetadata,
) -> SearchDocument {
    let title = std::iter::once(example.title.clone())
        .chain(metadata.related_symbols.iter().cloned())
        .chain(metadata.related_modules.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");

    let mut terms = vec![example.title.clone(), example.path.clone()];
    if let Some(summary) = &example.summary {
        terms.push(summary.clone());
    }
    terms.extend(metadata.related_symbols.iter().cloned());
    terms.extend(metadata.related_modules.iter().cloned());

    SearchDocument {
        id: example.example_id.clone(),
        title,
        kind: "example".to_string(),
        path: example.path.clone(),
        scope: example.repo_id.clone(),
        namespace: metadata
            .related_modules
            .first()
            .cloned()
            .unwrap_or_default(),
        terms,
    }
}

fn build_example_metadata_lookup(
    analysis: &RepositoryAnalysisOutput,
) -> BTreeMap<String, ExampleSearchMetadata> {
    let relation_lookup = example_relation_lookup(&analysis.relations);
    analysis
        .examples
        .iter()
        .map(|example| {
            (
                example.example_id.clone(),
                ExampleSearchMetadata {
                    related_symbols: related_symbols_for_example(
                        example.example_id.as_str(),
                        &relation_lookup,
                        &analysis.symbols,
                    ),
                    related_modules: related_modules_for_example(
                        example.example_id.as_str(),
                        &relation_lookup,
                        &analysis.modules,
                    ),
                },
            )
        })
        .collect()
}

fn raw_example_match_score(
    normalized_query: &str,
    example: &ExampleRecord,
    metadata: &ExampleSearchMetadata,
) -> Option<u8> {
    let title = example.title.to_ascii_lowercase();
    let path = example.path.to_ascii_lowercase();
    let summary = example
        .summary
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    example_match_score(
        normalized_query,
        title.as_str(),
        path.as_str(),
        summary.as_str(),
        &metadata.related_symbols,
        &metadata.related_modules,
    )
}

fn legacy_module_matches(
    normalized_query: &str,
    modules: &[ModuleRecord],
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    let mut matches = modules
        .iter()
        .filter_map(|module| {
            let qualified_name = module.qualified_name.to_ascii_lowercase();
            let path = module.path.to_ascii_lowercase();
            let score =
                module_match_score(normalized_query, qualified_name.as_str(), path.as_str())?;
            Some((score, module.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_module), (right_score, right_module)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_module.qualified_name.cmp(&right_module.qualified_name))
            .then_with(|| left_module.path.cmp(&right_module.path))
    });

    matches
        .into_iter()
        .map(|(raw_score, module)| RankedSearchRecord {
            item: module,
            score: normalized_rank_score(raw_score, MODULE_SEARCH_BUCKETS),
        })
        .collect()
}

fn legacy_symbol_matches(
    normalized_query: &str,
    symbols: &[SymbolRecord],
) -> Vec<RankedSearchRecord<SymbolRecord>> {
    let mut matches = symbols
        .iter()
        .filter_map(|symbol| {
            let name = symbol.name.to_ascii_lowercase();
            let qualified_name = symbol.qualified_name.to_ascii_lowercase();
            let path = symbol.path.to_ascii_lowercase();
            let signature = symbol
                .signature
                .as_deref()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            let score = symbol_match_score(
                normalized_query,
                name.as_str(),
                qualified_name.as_str(),
                path.as_str(),
                signature.as_str(),
            )?;
            Some((score, symbol.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_symbol), (right_score, right_symbol)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_symbol.name.cmp(&right_symbol.name))
            .then_with(|| left_symbol.qualified_name.cmp(&right_symbol.qualified_name))
            .then_with(|| left_symbol.path.cmp(&right_symbol.path))
    });

    matches
        .into_iter()
        .map(|(raw_score, symbol)| RankedSearchRecord {
            item: symbol,
            score: normalized_rank_score(raw_score, SYMBOL_SEARCH_BUCKETS),
        })
        .collect()
}

fn legacy_example_matches(
    normalized_query: &str,
    examples: &[ExampleRecord],
    metadata_lookup: &BTreeMap<String, ExampleSearchMetadata>,
) -> Vec<RankedSearchRecord<ExampleRecord>> {
    let mut matches = examples
        .iter()
        .filter_map(|example| {
            let metadata = metadata_lookup
                .get(example.example_id.as_str())
                .cloned()
                .unwrap_or_default();
            let score = raw_example_match_score(normalized_query, example, &metadata)?;
            Some((score, example.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_example), (right_score, right_example)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_example.title.cmp(&right_example.title))
            .then_with(|| left_example.path.cmp(&right_example.path))
    });

    matches
        .into_iter()
        .map(|(raw_score, example)| RankedSearchRecord {
            item: example,
            score: normalized_rank_score(raw_score, EXAMPLE_SEARCH_BUCKETS),
        })
        .collect()
}

fn indexed_module_exact_matches(
    index: &SearchDocumentIndex,
    lookup: &BTreeMap<String, ModuleRecord>,
    query: &str,
    normalized_query: &str,
    limit: usize,
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    let mut matches = index
        .search_exact(query, limit)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|document| {
            let module = lookup.get(document.id.as_str())?.clone();
            let qualified_name = module.qualified_name.to_ascii_lowercase();
            let path = module.path.to_ascii_lowercase();
            let raw_score =
                module_match_score(normalized_query, qualified_name.as_str(), path.as_str())?;
            Some((raw_score, module))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_module), (right_score, right_module)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_module.qualified_name.cmp(&right_module.qualified_name))
            .then_with(|| left_module.path.cmp(&right_module.path))
    });

    matches
        .into_iter()
        .map(|(raw_score, module)| RankedSearchRecord {
            item: module,
            score: normalized_rank_score(raw_score, MODULE_SEARCH_BUCKETS),
        })
        .collect()
}

fn indexed_symbol_exact_matches(
    index: &SearchDocumentIndex,
    lookup: &BTreeMap<String, SymbolRecord>,
    query: &str,
    normalized_query: &str,
    limit: usize,
) -> Vec<RankedSearchRecord<SymbolRecord>> {
    let mut matches = index
        .search_exact(query, limit)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|document| {
            let symbol = lookup.get(document.id.as_str())?.clone();
            let name = symbol.name.to_ascii_lowercase();
            let qualified_name = symbol.qualified_name.to_ascii_lowercase();
            let path = symbol.path.to_ascii_lowercase();
            let signature = symbol
                .signature
                .as_deref()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            let raw_score = symbol_match_score(
                normalized_query,
                name.as_str(),
                qualified_name.as_str(),
                path.as_str(),
                signature.as_str(),
            )?;
            Some((raw_score, symbol))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_symbol), (right_score, right_symbol)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_symbol.name.cmp(&right_symbol.name))
            .then_with(|| left_symbol.qualified_name.cmp(&right_symbol.qualified_name))
            .then_with(|| left_symbol.path.cmp(&right_symbol.path))
    });

    matches
        .into_iter()
        .map(|(raw_score, symbol)| RankedSearchRecord {
            item: symbol,
            score: normalized_rank_score(raw_score, SYMBOL_SEARCH_BUCKETS),
        })
        .collect()
}

fn indexed_example_exact_matches(
    index: &SearchDocumentIndex,
    lookup: &BTreeMap<String, ExampleRecord>,
    metadata_lookup: &BTreeMap<String, ExampleSearchMetadata>,
    query: &str,
    normalized_query: &str,
    limit: usize,
) -> Vec<RankedSearchRecord<ExampleRecord>> {
    let mut matches = index
        .search_exact(query, limit)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|document| {
            let example = lookup.get(document.id.as_str())?.clone();
            let metadata = metadata_lookup
                .get(example.example_id.as_str())
                .cloned()
                .unwrap_or_default();
            let raw_score = raw_example_match_score(normalized_query, &example, &metadata)?;
            Some((raw_score, example))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_example), (right_score, right_example)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_example.title.cmp(&right_example.title))
            .then_with(|| left_example.path.cmp(&right_example.path))
    });

    matches
        .into_iter()
        .map(|(raw_score, example)| RankedSearchRecord {
            item: example,
            score: normalized_rank_score(raw_score, EXAMPLE_SEARCH_BUCKETS),
        })
        .collect()
}

fn indexed_module_fuzzy_matches(
    index: &SearchDocumentIndex,
    lookup: &BTreeMap<String, ModuleRecord>,
    query: &str,
    limit: usize,
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    let mut matches = index
        .search_fuzzy(query, limit, FuzzySearchOptions::path_search())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|hit| {
            let module = lookup.get(hit.item.id.as_str())?.clone();
            Some((f64::from(hit.score), hit.distance, module))
        })
        .collect::<Vec<_>>();

    matches.sort_by(
        |(left_score, left_distance, left_module), (right_score, right_distance, right_module)| {
            right_score
                .partial_cmp(left_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left_distance.cmp(right_distance))
                .then_with(|| left_module.qualified_name.cmp(&right_module.qualified_name))
                .then_with(|| left_module.path.cmp(&right_module.path))
        },
    );

    matches
        .into_iter()
        .map(|(score, _distance, module)| RankedSearchRecord {
            item: module,
            score,
        })
        .collect()
}

fn indexed_symbol_fuzzy_matches(
    index: &SearchDocumentIndex,
    lookup: &BTreeMap<String, SymbolRecord>,
    query: &str,
    limit: usize,
) -> Vec<RankedSearchRecord<SymbolRecord>> {
    let mut matches = index
        .search_fuzzy(query, limit, FuzzySearchOptions::symbol_search())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|hit| {
            let symbol = lookup.get(hit.item.id.as_str())?.clone();
            Some((f64::from(hit.score), hit.distance, symbol))
        })
        .collect::<Vec<_>>();

    matches.sort_by(
        |(left_score, left_distance, left_symbol), (right_score, right_distance, right_symbol)| {
            right_score
                .partial_cmp(left_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left_distance.cmp(right_distance))
                .then_with(|| left_symbol.name.cmp(&right_symbol.name))
                .then_with(|| left_symbol.qualified_name.cmp(&right_symbol.qualified_name))
                .then_with(|| left_symbol.path.cmp(&right_symbol.path))
        },
    );

    matches
        .into_iter()
        .map(|(score, _distance, symbol)| RankedSearchRecord {
            item: symbol,
            score,
        })
        .collect()
}

fn indexed_example_fuzzy_matches(
    index: &SearchDocumentIndex,
    lookup: &BTreeMap<String, ExampleRecord>,
    query: &str,
    limit: usize,
) -> Vec<RankedSearchRecord<ExampleRecord>> {
    let mut matches = index
        .search_fuzzy(query, limit, FuzzySearchOptions::document_search())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|hit| {
            let example = lookup.get(hit.item.id.as_str())?.clone();
            Some((f64::from(hit.score), hit.distance, example))
        })
        .collect::<Vec<_>>();

    matches.sort_by(
        |(left_score, left_distance, left_example),
         (right_score, right_distance, right_example)| {
            right_score
                .partial_cmp(left_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left_distance.cmp(right_distance))
                .then_with(|| left_example.title.cmp(&right_example.title))
                .then_with(|| left_example.path.cmp(&right_example.path))
        },
    );

    matches
        .into_iter()
        .map(|(score, _distance, example)| RankedSearchRecord {
            item: example,
            score,
        })
        .collect()
}

fn ranked_module_matches(
    query: &str,
    modules: &[ModuleRecord],
    limit: usize,
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    if modules.is_empty() || limit == 0 {
        return Vec::new();
    }

    let normalized_query = query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return modules
            .iter()
            .take(limit)
            .cloned()
            .map(|module| RankedSearchRecord {
                item: module,
                score: normalized_rank_score(0, MODULE_SEARCH_BUCKETS),
            })
            .collect();
    }

    let search_limit = search_candidate_limit(limit);
    let lookup = modules
        .iter()
        .map(|module| (module.module_id.clone(), module.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();

    if let Some(index) = build_search_document_index(modules.iter().map(module_search_document)) {
        for candidate in indexed_module_exact_matches(
            &index,
            &lookup,
            query,
            normalized_query.as_str(),
            search_limit,
        ) {
            if seen_ids.insert(candidate.item.module_id.clone()) {
                ranked.push(candidate);
                if ranked.len() >= limit {
                    return ranked;
                }
            }
        }

        for candidate in indexed_module_fuzzy_matches(&index, &lookup, query, search_limit) {
            if seen_ids.insert(candidate.item.module_id.clone()) {
                ranked.push(candidate);
                if ranked.len() >= limit {
                    return ranked;
                }
            }
        }
    }

    for candidate in legacy_module_matches(normalized_query.as_str(), modules) {
        if seen_ids.insert(candidate.item.module_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                break;
            }
        }
    }

    ranked
}

fn ranked_symbol_matches(
    query: &str,
    symbols: &[SymbolRecord],
    limit: usize,
) -> Vec<RankedSearchRecord<SymbolRecord>> {
    if symbols.is_empty() || limit == 0 {
        return Vec::new();
    }

    let normalized_query = query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return symbols
            .iter()
            .take(limit)
            .cloned()
            .map(|symbol| RankedSearchRecord {
                item: symbol,
                score: normalized_rank_score(0, SYMBOL_SEARCH_BUCKETS),
            })
            .collect();
    }

    let search_limit = search_candidate_limit(limit);
    let lookup = symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.clone(), symbol.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();

    if let Some(index) = build_search_document_index(symbols.iter().map(symbol_search_document)) {
        for candidate in indexed_symbol_exact_matches(
            &index,
            &lookup,
            query,
            normalized_query.as_str(),
            search_limit,
        ) {
            if seen_ids.insert(candidate.item.symbol_id.clone()) {
                ranked.push(candidate);
                if ranked.len() >= limit {
                    return ranked;
                }
            }
        }

        for candidate in indexed_symbol_fuzzy_matches(&index, &lookup, query, search_limit) {
            if seen_ids.insert(candidate.item.symbol_id.clone()) {
                ranked.push(candidate);
                if ranked.len() >= limit {
                    return ranked;
                }
            }
        }
    }

    for candidate in legacy_symbol_matches(normalized_query.as_str(), symbols) {
        if seen_ids.insert(candidate.item.symbol_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                break;
            }
        }
    }

    ranked
}

fn ranked_example_matches(
    query: &str,
    examples: &[ExampleRecord],
    metadata_lookup: &BTreeMap<String, ExampleSearchMetadata>,
    limit: usize,
) -> Vec<RankedSearchRecord<ExampleRecord>> {
    if examples.is_empty() || limit == 0 {
        return Vec::new();
    }

    let normalized_query = query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return examples
            .iter()
            .take(limit)
            .cloned()
            .map(|example| RankedSearchRecord {
                item: example,
                score: normalized_rank_score(0, EXAMPLE_SEARCH_BUCKETS),
            })
            .collect();
    }

    let search_limit = search_candidate_limit(limit);
    let lookup = examples
        .iter()
        .map(|example| (example.example_id.clone(), example.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();

    if let Some(index) = build_search_document_index(examples.iter().map(|example| {
        let metadata = metadata_lookup
            .get(example.example_id.as_str())
            .cloned()
            .unwrap_or_default();
        example_search_document(example, &metadata)
    })) {
        for candidate in indexed_example_exact_matches(
            &index,
            &lookup,
            metadata_lookup,
            query,
            normalized_query.as_str(),
            search_limit,
        ) {
            if seen_ids.insert(candidate.item.example_id.clone()) {
                ranked.push(candidate);
                if ranked.len() >= limit {
                    return ranked;
                }
            }
        }

        for candidate in indexed_example_fuzzy_matches(&index, &lookup, query, search_limit) {
            if seen_ids.insert(candidate.item.example_id.clone()) {
                ranked.push(candidate);
                if ranked.len() >= limit {
                    return ranked;
                }
            }
        }
    }

    for candidate in legacy_example_matches(normalized_query.as_str(), examples, metadata_lookup) {
        if seen_ids.insert(candidate.item.example_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                break;
            }
        }
    }

    ranked
}

/// Build a repository overview result from normalized analysis records.
#[must_use]
pub fn build_repo_overview(
    query: &RepoOverviewQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoOverviewResult {
    let repository = analysis.repository.as_ref();
    RepoOverviewResult {
        repo_id: query.repo_id.clone(),
        display_name: repository.map_or_else(
            || query.repo_id.clone(),
            |repository| repository.name.clone(),
        ),
        revision: repository.and_then(|repository| repository.revision.clone()),
        module_count: analysis.modules.len(),
        symbol_count: analysis.symbols.len(),
        example_count: analysis.examples.len(),
        doc_count: analysis.docs.len(),
        hierarchical_uri: Some(repo_hierarchical_uri(query.repo_id.as_str())),
        hierarchy: Some(vec!["repo".to_string(), query.repo_id.clone()]),
    }
}

/// Load configuration, analyze one repository, and return its overview.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_overview_from_config_with_registry(
    query: &RepoOverviewQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoOverviewResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_overview(query, &analysis))
}

/// Load configuration, analyze one repository, and return its overview.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_overview_from_config(
    query: &RepoOverviewQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoOverviewResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_overview_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build a module search result from normalized analysis records.
#[must_use]
pub fn build_module_search(
    query: &ModuleSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> ModuleSearchResult {
    let limit = query.limit.max(1);
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let projection_lookup = projection_page_lookup(analysis);
    let saliency_map = super::super::saliency::compute_repository_saliency(analysis);
    let selected = ranked_module_matches(query.query.as_str(), &analysis.modules, limit);
    let modules = selected
        .iter()
        .map(|candidate| candidate.item.clone())
        .collect::<Vec<_>>();
    let module_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| {
            let normalized_score = candidate.score;
            let module = candidate.item;
            let module_id = module.module_id.clone();
            let module_path = module.path.clone();
            let (implicit_backlinks, implicit_backlink_items) =
                backlinks_for(module_id.as_str(), &backlink_lookup);
            let saliency_score = saliency_map.get(module_id.as_str()).copied();

            ModuleSearchHit {
                module,
                score: Some(normalized_score),
                rank: Some(index + 1),
                saliency_score,
                hierarchical_uri: Some(record_hierarchical_uri(
                    query.repo_id.as_str(),
                    infer_ecosystem(query.repo_id.as_str()),
                    "api",
                    module_path.as_str(),
                    module_id.as_str(),
                )),
                hierarchy: hierarchy_segments_from_path(module_path.as_str()),
                implicit_backlinks,
                implicit_backlink_items,
                projection_page_ids: projection_pages_for(module_id.as_str(), &projection_lookup),
            }
        })
        .collect::<Vec<_>>();

    ModuleSearchResult {
        repo_id: query.repo_id.clone(),
        modules,
        module_hits,
    }
}

/// Load configuration, analyze one repository, and return matching modules.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn module_search_from_config_with_registry(
    query: &ModuleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<ModuleSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_module_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching modules.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn module_search_from_config(
    query: &ModuleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<ModuleSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    module_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build a symbol search result from normalized analysis records.
#[must_use]
pub fn build_symbol_search(
    query: &SymbolSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> SymbolSearchResult {
    let limit = query.limit.max(1);
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let projection_lookup = projection_page_lookup(analysis);
    let saliency_map = super::super::saliency::compute_repository_saliency(analysis);
    let selected = ranked_symbol_matches(query.query.as_str(), &analysis.symbols, limit);
    let symbols = selected
        .iter()
        .map(|candidate| candidate.item.clone())
        .collect::<Vec<_>>();
    let symbol_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| {
            let normalized_score = candidate.score;
            let symbol = candidate.item;
            let audit_status = symbol.audit_status.clone();
            let verification_state = symbol.verification_state.clone().or_else(|| {
                audit_status.as_deref().map(|status| match status {
                    "verified" | "approved" => "verified".to_string(),
                    _ => "unverified".to_string(),
                })
            });
            let symbol_id = symbol.symbol_id.clone();
            let symbol_path = symbol.path.clone();
            let (implicit_backlinks, implicit_backlink_items) =
                backlinks_for(symbol_id.as_str(), &backlink_lookup);
            let saliency_score = saliency_map.get(symbol_id.as_str()).copied();

            SymbolSearchHit {
                symbol,
                score: Some(normalized_score),
                rank: Some(index + 1),
                saliency_score,
                hierarchical_uri: Some(record_hierarchical_uri(
                    query.repo_id.as_str(),
                    infer_ecosystem(query.repo_id.as_str()),
                    "api",
                    symbol_path.as_str(),
                    symbol_id.as_str(),
                )),
                hierarchy: hierarchy_segments_from_path(symbol_path.as_str()),
                implicit_backlinks,
                implicit_backlink_items,
                projection_page_ids: projection_pages_for(symbol_id.as_str(), &projection_lookup),
                audit_status,
                verification_state,
            }
        })
        .collect::<Vec<_>>();

    SymbolSearchResult {
        repo_id: query.repo_id.clone(),
        symbols,
        symbol_hits,
    }
}

/// Load configuration, analyze one repository, and return matching symbols.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn symbol_search_from_config_with_registry(
    query: &SymbolSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<SymbolSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_symbol_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching symbols.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn symbol_search_from_config(
    query: &SymbolSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<SymbolSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    symbol_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build an example search result from normalized analysis records.
#[must_use]
pub fn build_example_search(
    query: &ExampleSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> ExampleSearchResult {
    let limit = query.limit.max(1);
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let projection_lookup = projection_page_lookup(analysis);
    let metadata_lookup = build_example_metadata_lookup(analysis);
    let saliency_map = super::super::saliency::compute_repository_saliency(analysis);
    let selected = ranked_example_matches(
        query.query.as_str(),
        &analysis.examples,
        &metadata_lookup,
        limit,
    );
    let examples = selected
        .iter()
        .map(|candidate| candidate.item.clone())
        .collect::<Vec<_>>();
    let example_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| {
            let normalized_score = candidate.score;
            let example = candidate.item;
            let example_id = example.example_id.clone();
            let example_path = example.path.clone();
            let (implicit_backlinks, implicit_backlink_items) =
                backlinks_for(example_id.as_str(), &backlink_lookup);
            let saliency_score = saliency_map.get(example_id.as_str()).copied();

            ExampleSearchHit {
                example,
                score: Some(normalized_score),
                rank: Some(index + 1),
                saliency_score,
                hierarchical_uri: Some(record_hierarchical_uri(
                    query.repo_id.as_str(),
                    infer_ecosystem(query.repo_id.as_str()),
                    "examples",
                    example_path.as_str(),
                    example_id.as_str(),
                )),
                hierarchy: hierarchy_segments_from_path(example_path.as_str()),
                implicit_backlinks,
                implicit_backlink_items,
                projection_page_ids: projection_pages_for(example_id.as_str(), &projection_lookup),
            }
        })
        .collect::<Vec<_>>();

    ExampleSearchResult {
        repo_id: query.repo_id.clone(),
        examples,
        example_hits,
    }
}

/// Load configuration, analyze one repository, and return matching examples.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn example_search_from_config_with_registry(
    query: &ExampleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<ExampleSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_example_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching examples.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn example_search_from_config(
    query: &ExampleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<ExampleSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    example_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build an import search result from normalized analysis records.
#[must_use]
pub fn build_import_search(
    query: &ImportSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> ImportSearchResult {
    let limit = query.limit.max(1);
    let normalized_package = query.package.as_deref().map(str::to_ascii_lowercase);
    let normalized_module = query.module.as_deref().map(str::to_ascii_lowercase);

    let mut matches: Vec<(u8, &ImportRecord)> = analysis
        .imports
        .iter()
        .filter_map(|import| {
            let score = import_match_score(
                normalized_package.as_deref(),
                normalized_module.as_deref(),
                import,
            )?;
            Some((score, import))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, _left_import), (right_score, _right_import)| {
        left_score.cmp(right_score)
    });

    let selected = matches.into_iter().take(limit).collect::<Vec<_>>();
    let imports = selected
        .iter()
        .map(|(_score, import)| (*import).clone())
        .collect::<Vec<_>>();
    let import_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, (raw_score, import))| ImportSearchHit {
            import: import.clone(),
            score: Some(normalized_rank_score(raw_score, 3)),
            rank: Some(index + 1),
        })
        .collect::<Vec<_>>();

    ImportSearchResult {
        repo_id: query.repo_id.clone(),
        imports,
        import_hits,
    }
}

/// Load configuration, analyze one repository, and return matching imports.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn import_search_from_config_with_registry(
    query: &ImportSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<ImportSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_import_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching imports.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn import_search_from_config(
    query: &ImportSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<ImportSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    import_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build a documentation coverage result from normalized analysis records.
#[must_use]
pub fn build_doc_coverage(
    query: &DocCoverageQuery,
    analysis: &RepositoryAnalysisOutput,
) -> DocCoverageResult {
    let scoped_module = resolve_module_scope(query.module_id.as_deref(), &analysis.modules);
    let scoped_docs = docs_in_scope(scoped_module, analysis);
    let scoped_symbols = symbols_in_scope(scoped_module, &analysis.symbols);
    let covered_symbol_ids =
        documented_symbol_ids(scoped_module, &analysis.symbols, &analysis.relations);
    let covered_symbols = scoped_symbols
        .iter()
        .filter(|symbol| covered_symbol_ids.contains(symbol.symbol_id.as_str()))
        .count();

    DocCoverageResult {
        repo_id: query.repo_id.clone(),
        module_id: scoped_module
            .map(|module| module.module_id.clone())
            .or_else(|| query.module_id.clone()),
        docs: scoped_docs,
        covered_symbols,
        uncovered_symbols: scoped_symbols.len().saturating_sub(covered_symbols),
        hierarchical_uri: Some(repo_hierarchical_uri(query.repo_id.as_str())),
        hierarchy: Some(vec!["repo".to_string(), query.repo_id.clone()]),
    }
}

/// Load configuration, analyze one repository, and return documentation coverage.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn doc_coverage_from_config_with_registry(
    query: &DocCoverageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocCoverageResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_doc_coverage(query, &analysis))
}

/// Load configuration, analyze one repository, and return documentation coverage.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn doc_coverage_from_config(
    query: &DocCoverageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocCoverageResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    doc_coverage_from_config_with_registry(query, config_path, cwd, &registry)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::analyzers::{
        DocRecord, ExampleRecord, ModuleRecord, RelationKind, RelationRecord, RepoSymbolKind,
        RepositoryAnalysisOutput, RepositoryRecord, SymbolRecord,
    };

    #[allow(clippy::too_many_lines)]
    fn sample_search_analysis(repo_id: &str) -> RepositoryAnalysisOutput {
        let module_id = format!("repo:{repo_id}:module:ProjectionPkg");
        let solve_symbol_id = format!("repo:{repo_id}:symbol:ProjectionPkg.solve");
        let problem_symbol_id = format!("repo:{repo_id}:symbol:ProjectionPkg.Problem");
        let readme_doc_id = format!("repo:{repo_id}:doc:README.md");
        let solve_doc_id = format!("repo:{repo_id}:doc:src/ProjectionPkg.jl#symbol:solve");
        let problem_doc_id = format!("repo:{repo_id}:doc:src/ProjectionPkg.jl#symbol:Problem");
        let example_id = format!("repo:{repo_id}:example:examples/basic.jl");

        RepositoryAnalysisOutput {
            repository: Some(RepositoryRecord {
                repo_id: repo_id.to_string(),
                name: "ProjectionPkg".to_string(),
                path: format!("/virtual/repos/{repo_id}"),
                url: None,
                revision: Some("fixture".to_string()),
                version: Some("0.1.0".to_string()),
                uuid: None,
                dependencies: Vec::new(),
            }),
            modules: vec![ModuleRecord {
                repo_id: repo_id.to_string(),
                module_id: module_id.clone(),
                qualified_name: "ProjectionPkg".to_string(),
                path: "src/ProjectionPkg.jl".to_string(),
            }],
            symbols: vec![
                SymbolRecord {
                    repo_id: repo_id.to_string(),
                    symbol_id: solve_symbol_id.clone(),
                    module_id: Some(module_id.clone()),
                    name: "solve".to_string(),
                    qualified_name: "ProjectionPkg.solve".to_string(),
                    kind: RepoSymbolKind::Function,
                    path: "src/ProjectionPkg.jl".to_string(),
                    line_start: None,
                    line_end: None,
                    signature: Some("solve(problem::Problem)".to_string()),
                    audit_status: None,
                    verification_state: None,
                    attributes: BTreeMap::new(),
                },
                SymbolRecord {
                    repo_id: repo_id.to_string(),
                    symbol_id: problem_symbol_id.clone(),
                    module_id: Some(module_id.clone()),
                    name: "Problem".to_string(),
                    qualified_name: "ProjectionPkg.Problem".to_string(),
                    kind: RepoSymbolKind::Type,
                    path: "src/ProjectionPkg.jl".to_string(),
                    line_start: None,
                    line_end: None,
                    signature: Some("struct Problem".to_string()),
                    audit_status: None,
                    verification_state: None,
                    attributes: BTreeMap::new(),
                },
            ],
            imports: Vec::new(),
            examples: vec![ExampleRecord {
                repo_id: repo_id.to_string(),
                example_id: example_id.clone(),
                title: "basic".to_string(),
                path: "examples/basic.jl".to_string(),
                summary: Some("Solve a projection problem end to end.".to_string()),
            }],
            docs: vec![
                DocRecord {
                    repo_id: repo_id.to_string(),
                    doc_id: readme_doc_id.clone(),
                    title: "README.md".to_string(),
                    path: "README.md".to_string(),
                    format: Some("md".to_string()),
                },
                DocRecord {
                    repo_id: repo_id.to_string(),
                    doc_id: problem_doc_id.clone(),
                    title: "Problem".to_string(),
                    path: "src/ProjectionPkg.jl#symbol:Problem".to_string(),
                    format: Some("julia_docstring".to_string()),
                },
                DocRecord {
                    repo_id: repo_id.to_string(),
                    doc_id: solve_doc_id.clone(),
                    title: "solve".to_string(),
                    path: "src/ProjectionPkg.jl#symbol:solve".to_string(),
                    format: Some("julia_docstring".to_string()),
                },
            ],
            relations: vec![
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: readme_doc_id,
                    target_id: module_id.clone(),
                    kind: RelationKind::Documents,
                },
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: problem_doc_id,
                    target_id: problem_symbol_id,
                    kind: RelationKind::Documents,
                },
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: solve_doc_id,
                    target_id: solve_symbol_id.clone(),
                    kind: RelationKind::Documents,
                },
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: example_id.clone(),
                    target_id: module_id.clone(),
                    kind: RelationKind::ExampleOf,
                },
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: example_id,
                    target_id: solve_symbol_id,
                    kind: RelationKind::ExampleOf,
                },
            ],
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn module_search_uses_shared_tantivy_fuzzy_index_for_typos() {
        let analysis = sample_search_analysis("module-fuzzy");
        let result = build_module_search(
            &ModuleSearchQuery {
                repo_id: "module-fuzzy".to_string(),
                query: "ProjectonPkg".to_string(),
                limit: 10,
            },
            &analysis,
        );

        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].qualified_name, "ProjectionPkg");
        assert!(
            result.module_hits[0]
                .score
                .unwrap_or_else(|| panic!("shared fuzzy module search should emit a score"))
                > 0.0
        );
    }

    #[test]
    fn symbol_search_uses_shared_tantivy_fuzzy_index_for_typos() {
        let analysis = sample_search_analysis("symbol-fuzzy");
        let result = build_symbol_search(
            &SymbolSearchQuery {
                repo_id: "symbol-fuzzy".to_string(),
                query: "slove".to_string(),
                limit: 10,
            },
            &analysis,
        );

        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbols[0].name, "solve");
        assert!(
            result.symbol_hits[0]
                .score
                .unwrap_or_else(|| panic!("shared fuzzy symbol search should emit a score"))
                > 0.0
        );
    }

    #[test]
    fn example_search_uses_shared_tantivy_fuzzy_index_for_related_symbol_typos() {
        let analysis = sample_search_analysis("example-fuzzy");
        let result = build_example_search(
            &ExampleSearchQuery {
                repo_id: "example-fuzzy".to_string(),
                query: "slove".to_string(),
                limit: 10,
            },
            &analysis,
        );

        assert_eq!(result.examples.len(), 1);
        assert_eq!(result.examples[0].title, "basic");
        assert!(
            result.example_hits[0]
                .score
                .unwrap_or_else(|| panic!("shared fuzzy example search should emit a score"))
                > 0.0
        );
    }
}
