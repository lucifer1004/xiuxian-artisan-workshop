//! High-level repository intelligence service orchestration.

mod analysis;
mod bootstrap;
mod cached;
mod helpers;
mod merge;
mod projection;
mod registry;
mod relation_dedupe;
mod search;
mod sync;

pub use analysis::{
    analyze_registered_repository, analyze_registered_repository_with_registry,
    analyze_repository_from_config, analyze_repository_from_config_with_registry,
};
pub use bootstrap::bootstrap_builtin_registry;
pub use cached::analyze_registered_repository_cached_with_registry;
pub use helpers::relation_kind_label;
pub(crate) use helpers::{
    backlinks_for, documents_backlink_lookup, example_match_score, example_relation_lookup,
    hierarchy_segments_from_path, infer_ecosystem, module_match_score, normalized_rank_score,
    projection_page_lookup, projection_pages_for, record_hierarchical_uri,
    related_modules_for_example, related_symbols_for_example, symbol_match_score,
};

#[doc(hidden)]
const _: () = {
    let _ = (
        backlinks_for,
        documents_backlink_lookup,
        example_match_score,
        example_relation_lookup,
        hierarchy_segments_from_path,
        infer_ecosystem,
        module_match_score,
        normalized_rank_score,
        projection_page_lookup,
        projection_pages_for,
        record_hierarchical_uri,
        related_modules_for_example,
        related_symbols_for_example,
        symbol_match_score,
    );
};

pub use projection::*;
pub use registry::load_registered_repository;
pub use search::*;
pub use sync::*;

#[cfg(test)]
mod tests;
