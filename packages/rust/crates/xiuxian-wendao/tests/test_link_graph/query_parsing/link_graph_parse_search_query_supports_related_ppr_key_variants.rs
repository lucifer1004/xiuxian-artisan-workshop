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
fn test_link_graph_parse_search_query_supports_related_ppr_key_variants() {
    let parsed = parse_search_query(
        "related:seed related.ppr.alpha:0.75 related-ppr-max-iter:32 ppr_tol:1e-5 ppr-subgraph-mode:auto",
        LinkGraphSearchOptions::default(),
    );

    let related = parsed
        .options
        .filters
        .related
        .as_ref()
        .expect("expected related filter");
    assert_eq!(related.seeds, vec!["seed".to_string()]);
    let ppr = related.ppr.as_ref().expect("expected related ppr options");
    assert_eq!(ppr.alpha, Some(0.75));
    assert_eq!(ppr.max_iter, Some(32));
    assert_eq!(ppr.tol, Some(1e-5));
    assert_eq!(ppr.subgraph_mode, Some(LinkGraphPprSubgraphMode::Auto));
}
