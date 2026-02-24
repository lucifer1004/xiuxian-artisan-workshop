use super::super::super::state::ParsedDirectiveState;
use crate::link_graph::query::helpers::{
    parse_bool, parse_list_values, parse_tag_expression, push_unique_many,
};

pub(super) fn apply(
    key: &str,
    value: &str,
    negated_key: bool,
    state: &mut ParsedDirectiveState,
) -> bool {
    match key {
        "include" | "include_path" | "include_paths" | "path" => {
            if negated_key {
                push_unique_many(
                    &mut state.parsed_filters.exclude_paths,
                    parse_list_values(value),
                );
            } else {
                push_unique_many(
                    &mut state.parsed_filters.include_paths,
                    parse_list_values(value),
                );
            }
            true
        }
        "exclude" | "exclude_path" | "exclude_paths" => {
            if negated_key {
                push_unique_many(
                    &mut state.parsed_filters.include_paths,
                    parse_list_values(value),
                );
            } else {
                push_unique_many(
                    &mut state.parsed_filters.exclude_paths,
                    parse_list_values(value),
                );
            }
            true
        }
        "tag" | "tags" => {
            if negated_key {
                push_unique_many(&mut state.parsed_tags_not, parse_list_values(value));
            } else {
                parse_tag_expression(
                    value,
                    &mut state.parsed_tags_all,
                    &mut state.parsed_tags_any,
                    &mut state.parsed_tags_not,
                );
            }
            true
        }
        "tag_all" | "tags_all" => {
            if negated_key {
                push_unique_many(&mut state.parsed_tags_not, parse_list_values(value));
            } else {
                push_unique_many(&mut state.parsed_tags_all, parse_list_values(value));
            }
            true
        }
        "tag_any" | "tags_any" => {
            if negated_key {
                push_unique_many(&mut state.parsed_tags_not, parse_list_values(value));
            } else {
                push_unique_many(&mut state.parsed_tags_any, parse_list_values(value));
            }
            true
        }
        "tag_not" | "tags_not" => {
            if negated_key {
                push_unique_many(&mut state.parsed_tags_all, parse_list_values(value));
            } else {
                push_unique_many(&mut state.parsed_tags_not, parse_list_values(value));
            }
            true
        }
        "mentions_of" | "mention" | "mentions" => {
            push_unique_many(
                &mut state.parsed_filters.mentions_of,
                parse_list_values(value),
            );
            true
        }
        "mentioned_by" | "mentioned_by_notes" => {
            push_unique_many(
                &mut state.parsed_filters.mentioned_by_notes,
                parse_list_values(value),
            );
            true
        }
        "orphan" => {
            if let Some(flag) = parse_bool(value) {
                state.parsed_filters.orphan = flag;
            }
            true
        }
        "tagless" => {
            if let Some(flag) = parse_bool(value) {
                state.parsed_filters.tagless = flag;
            }
            true
        }
        "missing_backlink" => {
            if let Some(flag) = parse_bool(value) {
                state.parsed_filters.missing_backlink = flag;
            }
            true
        }
        _ => false,
    }
}
