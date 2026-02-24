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
fn test_knowledge_entry_with_metadata() {
    use serde_json::json;

    let mut entry = KnowledgeEntry::new(
        "metadata-test".to_string(),
        "With Metadata".to_string(),
        "Entry with extra metadata...".to_string(),
        KnowledgeCategory::Reference,
    );

    // Add metadata
    entry
        .metadata
        .insert("author".to_string(), json!("test-author"));
    entry.metadata.insert("reviewed".to_string(), json!(true));

    assert_eq!(entry.metadata.len(), 2);
    assert_eq!(entry.metadata.get("author"), Some(&json!("test-author")));
}
