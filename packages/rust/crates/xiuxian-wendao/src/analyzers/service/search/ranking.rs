use std::collections::{BTreeMap, HashSet};

use crate::analyzers::records::{ExampleRecord, ModuleRecord, SymbolRecord};

use super::super::helpers::normalized_rank_score;
use super::documents::{
    ExampleSearchMetadata, build_search_document_index, example_search_document,
    module_search_document, symbol_search_document,
};
use super::indexed_exact::{
    indexed_example_exact_matches, indexed_module_exact_matches, indexed_symbol_exact_matches,
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

pub(super) fn ranked_module_matches(
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

pub(super) fn ranked_symbol_matches(
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

pub(super) fn ranked_example_matches(
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
