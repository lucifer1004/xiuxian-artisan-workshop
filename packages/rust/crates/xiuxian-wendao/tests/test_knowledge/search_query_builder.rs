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
fn test_search_query_builder() {
    let query = KnowledgeSearchQuery::new("database error".to_string())
        .with_category(KnowledgeCategory::Error)
        .with_tags(vec!["sql".to_string(), "postgres".to_string()])
        .with_limit(10);

    assert_eq!(query.query, "database error");
    assert_eq!(query.category, Some(KnowledgeCategory::Error));
    assert_eq!(query.tags.len(), 2);
    assert_eq!(query.limit, 10);
}
