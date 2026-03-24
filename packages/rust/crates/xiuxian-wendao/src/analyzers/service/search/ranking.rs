use std::collections::{BTreeMap, HashSet};

use crate::analyzers::records::{ExampleRecord, ModuleRecord, SymbolRecord};
use crate::search::{FuzzyMatcher, FuzzySearchOptions, LexicalMatcher, SearchDocumentIndex};

use super::super::helpers::normalized_rank_score;
use super::documents::{
    ExampleSearchMetadata, build_search_document_index, example_search_document,
    module_search_document, symbol_search_document,
};
use super::indexed_exact::{
    indexed_example_exact_matches, indexed_example_prefix_matches, indexed_module_exact_matches,
    indexed_module_prefix_matches, indexed_symbol_exact_matches, indexed_symbol_prefix_matches,
};
use super::indexed_fuzzy::{
    indexed_example_fuzzy_matches, indexed_module_fuzzy_matches, indexed_symbol_fuzzy_matches,
};
use super::legacy::{legacy_example_matches, legacy_module_matches, legacy_symbol_matches};

pub(super) const MODULE_SEARCH_BUCKETS: u8 = 3;
pub(super) const SYMBOL_SEARCH_BUCKETS: u8 = 7;
pub(super) const EXAMPLE_SEARCH_BUCKETS: u8 = 10;
const SEARCH_CANDIDATE_MULTIPLIER: usize = 8;

#[derive(Debug, Clone)]
pub(super) struct RankedSearchRecord<T> {
    pub(super) item: T,
    pub(super) score: f64,
}

fn search_candidate_limit(limit: usize) -> usize {
    limit.max(1).saturating_mul(SEARCH_CANDIDATE_MULTIPLIER)
}

fn module_qualified_name(module: &ModuleRecord) -> &str {
    module.qualified_name.as_str()
}

fn lexical_module_fuzzy_matches(
    query: &str,
    modules: &[ModuleRecord],
    limit: usize,
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    let matcher = LexicalMatcher::new(
        modules,
        module_qualified_name,
        FuzzySearchOptions::camel_case_symbol(),
    );
    matcher
        .search(query, limit)
        .unwrap_or_default()
        .into_iter()
        .map(|matched_module| RankedSearchRecord {
            item: matched_module.item,
            score: f64::from(matched_module.score),
        })
        .collect()
}

pub(super) fn ranked_module_matches(
    query: &str,
    modules: &[ModuleRecord],
    limit: usize,
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    let lookup = modules
        .iter()
        .map(|module| (module.module_id.clone(), module.clone()))
        .collect::<BTreeMap<_, _>>();
    let Some(index) = build_search_document_index(modules.iter().map(module_search_document))
    else {
        return ranked_module_matches_without_index(query, modules, limit);
    };
    ranked_module_matches_from_index(query, modules, &lookup, &index, limit)
}

pub(super) fn ranked_module_matches_with_artifacts(
    query: &str,
    modules: &[ModuleRecord],
    lookup: &BTreeMap<String, ModuleRecord>,
    index: &SearchDocumentIndex,
    limit: usize,
) -> Vec<RankedSearchRecord<ModuleRecord>> {
    ranked_module_matches_from_index(query, modules, lookup, index, limit)
}

fn ranked_module_matches_from_index(
    query: &str,
    modules: &[ModuleRecord],
    lookup: &BTreeMap<String, ModuleRecord>,
    index: &SearchDocumentIndex,
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
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();

    for candidate in indexed_module_exact_matches(
        index,
        lookup,
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

    for candidate in indexed_module_prefix_matches(
        index,
        lookup,
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

    for candidate in indexed_module_fuzzy_matches(index, lookup, query, search_limit) {
        if seen_ids.insert(candidate.item.module_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                return ranked;
            }
        }
    }

    for candidate in lexical_module_fuzzy_matches(query, modules, search_limit) {
        if seen_ids.insert(candidate.item.module_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                return ranked;
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

fn ranked_module_matches_without_index(
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
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();
    for candidate in lexical_module_fuzzy_matches(query, modules, search_limit) {
        if seen_ids.insert(candidate.item.module_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                return ranked;
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

pub(super) fn ranked_symbol_matches(
    query: &str,
    symbols: &[SymbolRecord],
    limit: usize,
) -> Vec<RankedSearchRecord<SymbolRecord>> {
    let lookup = symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.clone(), symbol.clone()))
        .collect::<BTreeMap<_, _>>();
    let Some(index) = build_search_document_index(symbols.iter().map(symbol_search_document))
    else {
        return ranked_symbol_matches_without_index(query, symbols, limit);
    };
    ranked_symbol_matches_from_index(query, symbols, &lookup, &index, limit)
}

pub(super) fn ranked_symbol_matches_with_artifacts(
    query: &str,
    symbols: &[SymbolRecord],
    lookup: &BTreeMap<String, SymbolRecord>,
    index: &SearchDocumentIndex,
    limit: usize,
) -> Vec<RankedSearchRecord<SymbolRecord>> {
    ranked_symbol_matches_from_index(query, symbols, lookup, index, limit)
}

fn ranked_symbol_matches_from_index(
    query: &str,
    symbols: &[SymbolRecord],
    lookup: &BTreeMap<String, SymbolRecord>,
    index: &SearchDocumentIndex,
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
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();

    for candidate in indexed_symbol_exact_matches(
        index,
        lookup,
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

    for candidate in indexed_symbol_prefix_matches(
        index,
        lookup,
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

    for candidate in indexed_symbol_fuzzy_matches(index, lookup, query, search_limit) {
        if seen_ids.insert(candidate.item.symbol_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                return ranked;
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

fn ranked_symbol_matches_without_index(
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

    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();
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

pub(super) fn ranked_example_matches(
    query: &str,
    examples: &[ExampleRecord],
    metadata_lookup: &BTreeMap<String, ExampleSearchMetadata>,
    limit: usize,
) -> Vec<RankedSearchRecord<ExampleRecord>> {
    let lookup = examples
        .iter()
        .map(|example| (example.example_id.clone(), example.clone()))
        .collect::<BTreeMap<_, _>>();
    let Some(index) = build_search_document_index(examples.iter().map(|example| {
        let metadata = metadata_lookup
            .get(example.example_id.as_str())
            .cloned()
            .unwrap_or_default();
        example_search_document(example, &metadata)
    })) else {
        return ranked_example_matches_without_index(query, examples, metadata_lookup, limit);
    };
    ranked_example_matches_from_index(query, examples, metadata_lookup, &lookup, &index, limit)
}

pub(super) fn ranked_example_matches_with_artifacts(
    query: &str,
    examples: &[ExampleRecord],
    metadata_lookup: &BTreeMap<String, ExampleSearchMetadata>,
    lookup: &BTreeMap<String, ExampleRecord>,
    index: &SearchDocumentIndex,
    limit: usize,
) -> Vec<RankedSearchRecord<ExampleRecord>> {
    ranked_example_matches_from_index(query, examples, metadata_lookup, lookup, index, limit)
}

fn ranked_example_matches_from_index(
    query: &str,
    examples: &[ExampleRecord],
    metadata_lookup: &BTreeMap<String, ExampleSearchMetadata>,
    lookup: &BTreeMap<String, ExampleRecord>,
    index: &SearchDocumentIndex,
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
    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();

    for candidate in indexed_example_exact_matches(
        index,
        lookup,
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

    for candidate in indexed_example_prefix_matches(
        index,
        lookup,
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

    for candidate in indexed_example_fuzzy_matches(index, lookup, query, search_limit) {
        if seen_ids.insert(candidate.item.example_id.clone()) {
            ranked.push(candidate);
            if ranked.len() >= limit {
                return ranked;
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

fn ranked_example_matches_without_index(
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

    let mut ranked = Vec::new();
    let mut seen_ids = HashSet::new();
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
