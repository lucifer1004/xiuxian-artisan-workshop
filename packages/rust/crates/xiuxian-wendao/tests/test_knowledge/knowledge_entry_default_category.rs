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
fn test_knowledge_entry_default_category() {
    let entry = KnowledgeEntry::new(
        "test-004".to_string(),
        "Simple Note".to_string(),
        "Just a note...".to_string(),
        KnowledgeCategory::default(),
    );

    assert_eq!(entry.category, KnowledgeCategory::Note);
}
