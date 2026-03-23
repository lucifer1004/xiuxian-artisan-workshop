//! Core index build + query algorithms for markdown link graph.

use super::models::{
    LinkGraphDirection, LinkGraphDocument, LinkGraphEdgeType, LinkGraphHit, LinkGraphLinkFilter,
    LinkGraphMatchStrategy, LinkGraphMetadata, LinkGraphNeighbor, LinkGraphPprSubgraphMode,
    LinkGraphPromotedOverlayTelemetry, LinkGraphRelatedFilter, LinkGraphRelatedPprDiagnostics,
    LinkGraphRelatedPprOptions, LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions,
    LinkGraphSortField, LinkGraphSortOrder, LinkGraphSortTerm, LinkGraphStats, PageIndexNode,
};
use super::query::parse_search_query;

mod agentic_expansion;
mod agentic_overlay;
mod build;
mod constants;
mod ids;
mod lookup;
mod page_indices;
mod passages;
mod ppr;
mod rank;
mod scoring;
pub(crate) mod search;
mod semantic_documents;
mod shared;
mod symbol_cache;
mod traversal;
mod types;

pub use search::quantum_fusion::orchestrate::QuantumContextBuildError;
pub use search::quantum_fusion::semantic_ignition::{
    QuantumSemanticIgnition, QuantumSemanticIgnitionError, QuantumSemanticIgnitionFuture,
};

use constants::*;
use scoring::{
    normalize_with_case, score_document, score_document_exact, score_document_regex,
    score_path_fields, section_tree_distance, token_match_ratio, tokenize,
};
use shared::{
    ScoredSearchRow, deterministic_random_key, doc_contains_phrase, doc_sort_key,
    normalize_path_filter, path_matches_filter,
};
pub(crate) use types::{IndexedSection, SectionCandidate, SectionMatch};
pub use types::{
    LinkGraphCacheBuildMeta, LinkGraphIndex, LinkGraphRefreshMode, LinkGraphVirtualNode,
    PageIndexParent, SymbolCacheStats, SymbolRef,
};

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/index.rs"]
mod tests;
