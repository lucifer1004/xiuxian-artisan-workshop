//! Repository search functions (overview, module, symbol, example, import, doc coverage).
#[cfg(feature = "studio")]
mod artifacts;
mod contracts;
mod coverage;
mod documents;
mod example;
mod imports;
mod indexed_exact;
mod indexed_fuzzy;
mod legacy;
mod module;
mod overview;
mod ranking;
mod symbol;

#[cfg(test)]
#[path = "../../../../tests/unit/analyzers/service/search/mod.rs"]
mod tests;

pub use coverage::*;
pub use example::*;
pub use imports::*;
pub use module::*;
pub use overview::*;
pub use symbol::*;

#[cfg(feature = "studio")]
pub(crate) use artifacts::repository_search_artifacts;
#[cfg(feature = "studio")]
pub(crate) use contracts::{
    RepoAnalysisFallbackContract, example_fallback_contract, import_fallback_contract,
    module_fallback_contract, symbol_fallback_contract,
};
#[cfg(feature = "search-runtime")]
pub(crate) use contracts::canonical_import_query_text;
#[cfg(feature = "studio")]
pub(crate) use documents::ExampleSearchMetadata;
