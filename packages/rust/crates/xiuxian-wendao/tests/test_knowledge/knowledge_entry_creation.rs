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
use super::*;

#[test]
fn test_knowledge_entry_creation() {
    let entry = KnowledgeEntry::new(
        "test-001".to_string(),
        "Error Handling Pattern".to_string(),
        "Best practices for error handling in Rust...".to_string(),
        KnowledgeCategory::Pattern,
    );

    assert_eq!(entry.id, "test-001");
    assert_eq!(entry.title, "Error Handling Pattern");
    assert_eq!(entry.category, KnowledgeCategory::Pattern);
    assert!(entry.tags.is_empty());
    assert!(entry.source.is_none());
    assert_eq!(entry.version, 1);
}
