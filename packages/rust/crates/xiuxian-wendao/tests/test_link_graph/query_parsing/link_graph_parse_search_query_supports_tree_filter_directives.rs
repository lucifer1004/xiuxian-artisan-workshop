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
fn test_link_graph_parse_search_query_supports_tree_filter_directives() {
    let parsed = parse_search_query(
        "scope:section_only edge_types:structural,verified max_heading_level:3 max_tree_hops:2 collapse_to_doc:false per_doc_section_cap:4 min_section_words:18 architecture",
        LinkGraphSearchOptions::default(),
    );

    assert_eq!(parsed.query, "architecture");
    assert_eq!(
        parsed.options.filters.scope,
        Some(LinkGraphScope::SectionOnly)
    );
    assert_eq!(
        parsed.options.filters.edge_types,
        vec![LinkGraphEdgeType::Structural, LinkGraphEdgeType::Verified]
    );
    assert_eq!(parsed.options.filters.max_heading_level, Some(3));
    assert_eq!(parsed.options.filters.max_tree_hops, Some(2));
    assert_eq!(parsed.options.filters.collapse_to_doc, Some(false));
    assert_eq!(parsed.options.filters.per_doc_section_cap, Some(4));
    assert_eq!(parsed.options.filters.min_section_words, Some(18));
}
