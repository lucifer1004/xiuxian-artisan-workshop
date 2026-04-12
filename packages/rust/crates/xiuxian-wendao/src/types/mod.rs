//! Knowledge types - `KnowledgeEntry`, `KnowledgeCategory`, and related types.

mod query;
mod stats;

pub use query::KnowledgeSearchQuery;
pub use stats::KnowledgeStats;
pub use xiuxian_wendao_core::{KnowledgeCategory, KnowledgeEntry};
