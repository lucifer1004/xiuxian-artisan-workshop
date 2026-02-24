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
fn test_knowledge_entry_equality() {
    let entry1 = KnowledgeEntry {
        id: "same-id".to_string(),
        title: "Title".to_string(),
        content: "Content".to_string(),
        category: KnowledgeCategory::Note,
        tags: vec!["tag1".to_string()],
        source: None,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        version: 1,
        metadata: std::collections::HashMap::new(),
    };

    let entry2 = KnowledgeEntry {
        id: "same-id".to_string(),
        title: "Title".to_string(),
        content: "Content".to_string(),
        category: KnowledgeCategory::Note,
        tags: vec!["tag1".to_string()],
        source: None,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        version: 1,
        metadata: std::collections::HashMap::new(),
    };

    assert_eq!(entry1, entry2);
}
