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

mod link_graph_parse_search_query_does_not_infer_regex_from_plain_parentheses;
mod link_graph_parse_search_query_infers_regex_from_regex_markers;
mod link_graph_parse_search_query_keeps_fts_for_extension_only_query;
mod link_graph_parse_search_query_supports_directives_and_time_filters;
mod link_graph_parse_search_query_supports_limit_directive;
mod link_graph_parse_search_query_supports_multi_sort_terms_in_directive;
mod link_graph_parse_search_query_supports_negated_directives_and_pipe_values;
mod link_graph_parse_search_query_supports_parenthesized_boolean_tags;
mod link_graph_parse_search_query_supports_query_directive;
mod link_graph_parse_search_query_supports_related_ppr_key_variants;
mod link_graph_parse_search_query_supports_tree_filter_directives;
