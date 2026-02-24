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

mod link_graph_search_edge_type_filter_allows_verified_for_graph_filters;
mod link_graph_search_edge_type_filter_restricts_semantic_graph_filters;
mod link_graph_search_edge_type_filter_restricts_structural_scope;
mod link_graph_search_mixed_scope_collapse_toggle_changes_output_shape;
mod link_graph_search_options_deserialize_accepts_tree_filters;
mod link_graph_search_options_validate_rejects_invalid_tree_filters;
mod link_graph_search_section_scope_respects_per_doc_cap;
mod link_graph_search_tree_hops_limit_section_expansion;
mod link_graph_search_tree_level_and_min_words_filters;
