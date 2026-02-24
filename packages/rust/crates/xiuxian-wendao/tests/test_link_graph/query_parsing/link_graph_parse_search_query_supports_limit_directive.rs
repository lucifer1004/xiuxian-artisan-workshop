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
fn test_link_graph_parse_search_query_supports_limit_directive() {
    let parsed = parse_search_query(
        "query:hard constraints limit:8 scope:section_only",
        LinkGraphSearchOptions::default(),
    );

    assert_eq!(parsed.query, "hard constraints");
    assert_eq!(parsed.limit_override, Some(8));
    assert_eq!(
        parsed.options.filters.scope,
        Some(LinkGraphScope::SectionOnly)
    );
}
