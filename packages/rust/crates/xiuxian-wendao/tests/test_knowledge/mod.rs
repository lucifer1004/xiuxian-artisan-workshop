#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
//! Tests for xiuxian-wendao crate.

use xiuxian_wendao::{KnowledgeCategory, KnowledgeEntry, KnowledgeSearchQuery, KnowledgeStats};

mod knowledge_category_equality;
/// Test KnowledgeCategory enum variants.
mod knowledge_category_variants;
mod knowledge_entry_clone;
mod knowledge_entry_creation;
mod knowledge_entry_default_category;
mod knowledge_entry_equality;
mod knowledge_entry_tag_operations;
mod knowledge_entry_with_metadata;
mod knowledge_entry_with_options;
mod knowledge_stats_default;
mod search_query_builder;
mod search_query_creation;
mod search_query_default;
