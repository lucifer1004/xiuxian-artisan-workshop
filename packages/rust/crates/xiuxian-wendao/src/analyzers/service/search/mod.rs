//! Repository search functions (overview, module, symbol, example, import, doc coverage).
mod artifacts;
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
mod tests;

pub use coverage::*;
pub use example::*;
pub use imports::*;
pub use module::*;
pub use overview::*;
pub use symbol::*;

pub(crate) use artifacts::repository_search_artifacts;
pub(crate) use documents::ExampleSearchMetadata;
