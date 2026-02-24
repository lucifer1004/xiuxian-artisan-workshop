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
fn test_knowledge_entry_with_options() {
    let entry = KnowledgeEntry::new(
        "test-002".to_string(),
        "Async Error Handling".to_string(),
        "Handling errors in async Rust code...".to_string(),
        KnowledgeCategory::Technique,
    )
    .with_tags(vec![
        "async".to_string(),
        "error".to_string(),
        "rust".to_string(),
    ])
    .with_source(Some("docs/async-errors.md".to_string()));

    assert_eq!(entry.tags.len(), 3);
    assert_eq!(entry.source, Some("docs/async-errors.md".to_string()));
}
