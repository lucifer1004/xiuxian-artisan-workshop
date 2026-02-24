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
fn test_link_graph_parse_search_query_supports_parenthesized_boolean_tags() {
    let parsed = parse_search_query(
        "tag:(core OR infra) roadmap",
        LinkGraphSearchOptions::default(),
    );

    assert_eq!(parsed.query, "roadmap");
    let tags = parsed.options.filters.tags.expect("expected tags filter");
    assert!(tags.all.is_empty());
    assert_eq!(tags.any, vec!["core".to_string(), "infra".to_string()]);
    assert!(tags.not_tags.is_empty());
}
