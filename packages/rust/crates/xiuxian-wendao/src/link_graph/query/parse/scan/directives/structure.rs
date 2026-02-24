use super::super::super::state::ParsedDirectiveState;
use crate::link_graph::query::helpers::{
    parse_bool, parse_edge_type, parse_list_values, parse_scope, parse_timestamp,
};

pub(super) fn apply(key: &str, value: &str, state: &mut ParsedDirectiveState) -> bool {
    match key {
        "scope" => {
            state.parsed_scope = parse_scope(value);
            true
        }
        "max_heading_level" | "heading_level" => {
            if let Ok(level) = value.parse::<usize>()
                && (1..=6).contains(&level)
            {
                state.parsed_max_heading_level = Some(level);
            }
            true
        }
        "max_tree_hops" | "tree_hops" => {
            if let Ok(hops) = value.parse::<usize>() {
                state.parsed_max_tree_hops = Some(hops);
            }
            true
        }
        "collapse_to_doc" => {
            state.parsed_collapse_to_doc = parse_bool(value);
            true
        }
        "edge_type" | "edge_types" => {
            for item in parse_list_values(value) {
                if let Some(edge_type) = parse_edge_type(&item)
                    && !state.parsed_edge_types.contains(&edge_type)
                {
                    state.parsed_edge_types.push(edge_type);
                }
            }
            true
        }
        "per_doc_section_cap" => {
            if let Ok(cap) = value.parse::<usize>()
                && cap > 0
            {
                state.parsed_per_doc_section_cap = Some(cap);
            }
            true
        }
        "min_section_words" => {
            if let Ok(words) = value.parse::<usize>() {
                state.parsed_min_section_words = Some(words);
            }
            true
        }
        "created_after" => {
            state.parsed_created_after = parse_timestamp(value);
            true
        }
        "created_before" => {
            state.parsed_created_before = parse_timestamp(value);
            true
        }
        "modified_after" | "updated_after" => {
            state.parsed_modified_after = parse_timestamp(value);
            true
        }
        "modified_before" | "updated_before" => {
            state.parsed_modified_before = parse_timestamp(value);
            true
        }
        _ => false,
    }
}
