//! Block-level granularity for semantic addressing.

mod address;
mod block;
mod kind;
mod matching;

pub use address::{BlockAddress, BlockKindSpecifier};
pub use block::MarkdownBlock;
pub use kind::MarkdownBlockKind;
pub(crate) use matching::markdown_block_matches_kind;

#[cfg(test)]
#[path = "../../../../../tests/unit/link_graph/models/records/markdown_block.rs"]
mod tests;
