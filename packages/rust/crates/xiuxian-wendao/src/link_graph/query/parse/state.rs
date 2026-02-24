use crate::link_graph::models::{
    LinkGraphEdgeType, LinkGraphLinkFilter, LinkGraphMatchStrategy, LinkGraphPprSubgraphMode,
    LinkGraphRelatedFilter, LinkGraphRelatedPprOptions, LinkGraphScope, LinkGraphSearchFilters,
    LinkGraphSortTerm,
};

#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_field_names)]
pub(super) struct ParsedDirectiveState {
    pub parsed_match_strategy: Option<LinkGraphMatchStrategy>,
    pub parsed_sort_terms: Vec<LinkGraphSortTerm>,
    pub parsed_case_sensitive: Option<bool>,
    pub parsed_limit_override: Option<usize>,

    pub parsed_filters: LinkGraphSearchFilters,
    pub parsed_tags_all: Vec<String>,
    pub parsed_tags_any: Vec<String>,
    pub parsed_tags_not: Vec<String>,
    pub parsed_link_to: LinkGraphLinkFilter,
    pub parsed_linked_by: LinkGraphLinkFilter,
    pub parsed_related: LinkGraphRelatedFilter,
    pub parsed_related_ppr: LinkGraphRelatedPprOptions,
    pub parsed_scope: Option<LinkGraphScope>,
    pub parsed_max_heading_level: Option<usize>,
    pub parsed_max_tree_hops: Option<usize>,
    pub parsed_collapse_to_doc: Option<bool>,
    pub parsed_edge_types: Vec<LinkGraphEdgeType>,
    pub parsed_per_doc_section_cap: Option<usize>,
    pub parsed_min_section_words: Option<usize>,

    pub parsed_created_after: Option<i64>,
    pub parsed_created_before: Option<i64>,
    pub parsed_modified_after: Option<i64>,
    pub parsed_modified_before: Option<i64>,
}

pub(super) fn parse_ppr_subgraph_mode(raw: &str) -> Option<LinkGraphPprSubgraphMode> {
    match raw.trim().to_lowercase().as_str() {
        "auto" => Some(LinkGraphPprSubgraphMode::Auto),
        "disabled" => Some(LinkGraphPprSubgraphMode::Disabled),
        "force" => Some(LinkGraphPprSubgraphMode::Force),
        _ => None,
    }
}

pub(super) fn has_related_ppr_options(value: &LinkGraphRelatedPprOptions) -> bool {
    value.alpha.is_some()
        || value.max_iter.is_some()
        || value.tol.is_some()
        || value.subgraph_mode.is_some()
}
