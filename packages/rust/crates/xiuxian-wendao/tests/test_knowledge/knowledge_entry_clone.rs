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
fn test_knowledge_entry_clone() {
    let entry = KnowledgeEntry::new(
        "clone-test".to_string(),
        "Clone This".to_string(),
        "Content to clone...".to_string(),
        KnowledgeCategory::Solution,
    )
    .with_tags(vec!["clone".to_string()])
    .with_source(Some("clone.md".to_string()));

    let cloned = entry.clone();

    assert_eq!(entry.id, cloned.id);
    assert_eq!(entry.title, cloned.title);
    assert_eq!(entry.content, cloned.content);
    assert_eq!(entry.category, cloned.category);
    assert_eq!(entry.tags, cloned.tags);
    assert_eq!(entry.source, cloned.source);
}
