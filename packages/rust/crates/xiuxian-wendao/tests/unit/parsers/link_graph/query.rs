use crate::link_graph::{
    LinkGraphMatchStrategy, LinkGraphPprSubgraphMode, LinkGraphScope, LinkGraphSearchOptions,
    LinkGraphSortField, LinkGraphSortOrder,
};
use crate::parsers::link_graph::query::parse_search_query;

#[test]
fn canonical_parser_namespace_parses_query_directives_into_search_options() {
    let parsed = parse_search_query(
        "query:\"alpha beta\" limit:7 sort:path/asc scope:section case:true",
        LinkGraphSearchOptions::default(),
    );

    assert_eq!(parsed.query, "alpha beta");
    assert_eq!(parsed.limit_override, Some(7));
    assert_eq!(
        parsed.options.filters.scope,
        Some(LinkGraphScope::SectionOnly)
    );
    assert!(parsed.options.case_sensitive);
    assert_eq!(parsed.options.sort_terms.len(), 1);
    assert_eq!(parsed.options.sort_terms[0].field, LinkGraphSortField::Path);
    assert_eq!(parsed.options.sort_terms[0].order, LinkGraphSortOrder::Asc);
}

#[test]
fn parse_search_query_tracks_related_ppr_and_tree_filters() {
    let parsed = parse_search_query(
        "related:alpha~2 ppr_alpha:0.2 ppr_max_iter:9 ppr_tol:0.001 ppr_subgraph_mode:force edge_type:semantic max_tree_hops:3",
        LinkGraphSearchOptions::default(),
    );

    let related = parsed
        .options
        .filters
        .related
        .expect("related filter should parse");
    assert_eq!(related.seeds, vec!["alpha"]);
    assert_eq!(related.max_distance, Some(2));
    let ppr = related.ppr.expect("ppr options should parse");
    assert_eq!(ppr.alpha, Some(0.2));
    assert_eq!(ppr.max_iter, Some(9));
    assert_eq!(ppr.tol, Some(0.001));
    assert_eq!(ppr.subgraph_mode, Some(LinkGraphPprSubgraphMode::Force));
    assert_eq!(parsed.options.filters.edge_types.len(), 1);
    assert_eq!(parsed.options.filters.max_tree_hops, Some(3));
}

#[test]
fn parse_search_query_supports_direct_id_short_circuit() {
    let parsed = parse_search_query("id:docs/alpha", LinkGraphSearchOptions::default());

    assert_eq!(parsed.direct_id.as_deref(), Some("docs/alpha"));
    assert!(parsed.query.is_empty());
}

#[test]
fn parse_search_query_infers_exact_strategy_for_machine_like_residual() {
    let parsed = parse_search_query("notes/system-42.md", LinkGraphSearchOptions::default());

    assert_eq!(parsed.query, "notes/system-42.md");
    assert_eq!(parsed.options.match_strategy, LinkGraphMatchStrategy::Exact);
}

#[test]
fn parse_search_query_supports_negated_paths_and_boolean_tags() {
    let parsed = parse_search_query(
        "tag:(alpha OR beta) !path:archive,legacy",
        LinkGraphSearchOptions::default(),
    );

    let tags = parsed
        .options
        .filters
        .tags
        .expect("tag filter should parse");
    assert_eq!(tags.any, vec!["alpha", "beta"]);
    assert_eq!(
        parsed.options.filters.exclude_paths,
        vec!["archive", "legacy"]
    );
}
