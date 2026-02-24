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
fn test_link_graph_parse_search_query_supports_negated_directives_and_pipe_values() {
    let parsed = parse_search_query(
        "-tag:legacy -to:archive to:hub|index from:a|b",
        LinkGraphSearchOptions::default(),
    );

    assert_eq!(parsed.query, "");
    let tags = parsed.options.filters.tags.expect("expected tags filter");
    assert_eq!(tags.not_tags, vec!["legacy".to_string()]);

    let link_to = parsed
        .options
        .filters
        .link_to
        .expect("expected link_to filter");
    assert!(link_to.negate);
    assert_eq!(
        link_to.seeds,
        vec![
            "archive".to_string(),
            "hub".to_string(),
            "index".to_string()
        ]
    );

    let linked_by = parsed
        .options
        .filters
        .linked_by
        .expect("expected linked_by filter");
    assert_eq!(linked_by.seeds, vec!["a".to_string(), "b".to_string()]);
}
