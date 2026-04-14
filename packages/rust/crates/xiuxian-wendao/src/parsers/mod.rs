//! Canonical parser families for Wendao domain-core document understanding.

/// Docs-governance parser helpers.
#[cfg(any(test, feature = "studio", feature = "zhenfa-router"))]
pub(crate) mod docs_governance;
/// Graph persistence parsing.
pub mod graph;
/// Language-specific parser families.
pub mod languages;
pub mod link_graph;
pub mod markdown;
/// Search query parsing.
pub mod search;
/// Semantic-check grammar helpers.
#[cfg(any(test, feature = "zhenfa-router"))]
pub(crate) mod semantic_check;
pub mod zhixing;
