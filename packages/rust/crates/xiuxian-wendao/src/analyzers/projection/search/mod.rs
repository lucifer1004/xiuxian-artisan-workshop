mod heuristic;
mod indexed;
mod lexical;
mod mapping;
mod options;
mod ranking;
mod sort;

pub(crate) use indexed::build_projected_page_search_index;
#[cfg(feature = "studio")]
pub(crate) use ranking::build_repo_projected_page_search_with_artifacts;
pub use ranking::{build_repo_projected_page_search, scored_projected_page_matches};

#[allow(unused_imports)]
pub use ranking::build_repo_projected_page_search_with_options;

#[cfg(test)]
#[path = "../../../../tests/unit/analyzers/projection/search/mod.rs"]
mod tests;
