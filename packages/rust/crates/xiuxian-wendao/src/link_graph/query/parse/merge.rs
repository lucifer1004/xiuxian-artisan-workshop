use super::state::{ParsedDirectiveState, has_related_ppr_options};
use crate::link_graph::models::{
    LinkGraphMatchStrategy, LinkGraphSearchOptions, LinkGraphSortTerm, LinkGraphTagFilter,
};
use crate::link_graph::query::helpers::{infer_strategy_from_residual, is_default_sort_terms};

fn merge_match_strategy(
    base: &mut LinkGraphSearchOptions,
    residual_terms: &[String],
    state: &ParsedDirectiveState,
) {
    if base.match_strategy != LinkGraphMatchStrategy::Fts {
        return;
    }
    if let Some(strategy) = state.parsed_match_strategy {
        base.match_strategy = strategy;
        return;
    }
    let residual = residual_terms.join(" ");
    if let Some(inferred) = infer_strategy_from_residual(&residual) {
        base.match_strategy = inferred;
    }
}

fn merge_case_and_sort(base: &mut LinkGraphSearchOptions, state: &ParsedDirectiveState) {
    if !base.case_sensitive
        && let Some(case_sensitive) = state.parsed_case_sensitive
    {
        base.case_sensitive = case_sensitive;
    }
    if is_default_sort_terms(&base.sort_terms) && !state.parsed_sort_terms.is_empty() {
        base.sort_terms.clone_from(&state.parsed_sort_terms);
    }
    if base.sort_terms.is_empty() {
        base.sort_terms = vec![LinkGraphSortTerm::default()];
    }
}

fn merge_tag_and_link_filters(base: &mut LinkGraphSearchOptions, state: &ParsedDirectiveState) {
    if !state.parsed_tags_all.is_empty()
        || !state.parsed_tags_any.is_empty()
        || !state.parsed_tags_not.is_empty()
    {
        let parsed_tag_filter = LinkGraphTagFilter {
            all: state.parsed_tags_all.clone(),
            any: state.parsed_tags_any.clone(),
            not_tags: state.parsed_tags_not.clone(),
        };
        if base.filters.tags.is_none() {
            base.filters.tags = Some(parsed_tag_filter);
        }
    }
    if !state.parsed_link_to.seeds.is_empty() && base.filters.link_to.is_none() {
        base.filters.link_to = Some(state.parsed_link_to.clone());
    }
    if !state.parsed_linked_by.seeds.is_empty() && base.filters.linked_by.is_none() {
        base.filters.linked_by = Some(state.parsed_linked_by.clone());
    }
}

fn merge_related_filters(base: &mut LinkGraphSearchOptions, state: &ParsedDirectiveState) {
    let parsed_related_has_ppr = has_related_ppr_options(&state.parsed_related_ppr);
    if base.filters.related.is_none() {
        if !state.parsed_related.seeds.is_empty() {
            let mut related = state.parsed_related.clone();
            if parsed_related_has_ppr {
                related.ppr = Some(state.parsed_related_ppr.clone());
            }
            base.filters.related = Some(related);
        }
    } else if let Some(base_related) = base.filters.related.as_mut() {
        if base_related.max_distance.is_none() && state.parsed_related.max_distance.is_some() {
            base_related.max_distance = state.parsed_related.max_distance;
        }
        if base_related.ppr.is_none() && parsed_related_has_ppr {
            base_related.ppr = Some(state.parsed_related_ppr.clone());
        }
    }
}

fn merge_search_filters(base: &mut LinkGraphSearchOptions, state: &ParsedDirectiveState) {
    if base.filters.include_paths.is_empty() && !state.parsed_filters.include_paths.is_empty() {
        base.filters
            .include_paths
            .clone_from(&state.parsed_filters.include_paths);
    }
    if base.filters.exclude_paths.is_empty() && !state.parsed_filters.exclude_paths.is_empty() {
        base.filters
            .exclude_paths
            .clone_from(&state.parsed_filters.exclude_paths);
    }
    if base.filters.mentions_of.is_empty() && !state.parsed_filters.mentions_of.is_empty() {
        base.filters
            .mentions_of
            .clone_from(&state.parsed_filters.mentions_of);
    }
    if base.filters.mentioned_by_notes.is_empty()
        && !state.parsed_filters.mentioned_by_notes.is_empty()
    {
        base.filters
            .mentioned_by_notes
            .clone_from(&state.parsed_filters.mentioned_by_notes);
    }
    if !base.filters.orphan && state.parsed_filters.orphan {
        base.filters.orphan = true;
    }
    if !base.filters.tagless && state.parsed_filters.tagless {
        base.filters.tagless = true;
    }
    if !base.filters.missing_backlink && state.parsed_filters.missing_backlink {
        base.filters.missing_backlink = true;
    }
    if base.filters.scope.is_none() {
        base.filters.scope = state.parsed_scope;
    }
    if base.filters.max_heading_level.is_none() {
        base.filters.max_heading_level = state.parsed_max_heading_level;
    }
    if base.filters.max_tree_hops.is_none() {
        base.filters.max_tree_hops = state.parsed_max_tree_hops;
    }
    if base.filters.collapse_to_doc.is_none() {
        base.filters.collapse_to_doc = state.parsed_collapse_to_doc;
    }
    if base.filters.edge_types.is_empty() && !state.parsed_edge_types.is_empty() {
        base.filters.edge_types.clone_from(&state.parsed_edge_types);
    }
    if base.filters.per_doc_section_cap.is_none() {
        base.filters.per_doc_section_cap = state.parsed_per_doc_section_cap;
    }
    if base.filters.min_section_words.is_none() {
        base.filters.min_section_words = state.parsed_min_section_words;
    }
}

fn merge_time_filters(base: &mut LinkGraphSearchOptions, state: &ParsedDirectiveState) {
    if base.created_after.is_none() {
        base.created_after = state.parsed_created_after;
    }
    if base.created_before.is_none() {
        base.created_before = state.parsed_created_before;
    }
    if base.modified_after.is_none() {
        base.modified_after = state.parsed_modified_after;
    }
    if base.modified_before.is_none() {
        base.modified_before = state.parsed_modified_before;
    }
}

pub(super) fn merge_into_base(
    base: &mut LinkGraphSearchOptions,
    residual_terms: &[String],
    state: &ParsedDirectiveState,
) {
    merge_match_strategy(base, residual_terms, state);
    merge_case_and_sort(base, state);
    merge_tag_and_link_filters(base, state);
    merge_related_filters(base, state);
    merge_search_filters(base, state);
    merge_time_filters(base, state);
}
