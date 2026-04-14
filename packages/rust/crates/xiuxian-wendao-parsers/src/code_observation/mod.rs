//! Parser-owned Markdown `:OBSERVE:` parsing and scoped-matching helpers.

mod extract;
mod format;
mod glob;
mod types;

pub use extract::extract_observations;
pub use glob::path_matches_scope;
pub use types::CodeObservation;
