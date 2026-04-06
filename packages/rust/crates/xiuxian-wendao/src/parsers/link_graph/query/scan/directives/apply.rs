use super::{filters, links, search, structure};
use crate::parsers::link_graph::query::state::ParsedDirectiveState;

pub(in crate::parsers::link_graph::query::scan) fn apply_directive(
    key: &str,
    value: &str,
    negated_key: bool,
    state: &mut ParsedDirectiveState,
    residual_terms: &mut Vec<String>,
) -> bool {
    search::apply(key, value, state, residual_terms)
        || links::apply(key, value, negated_key, state)
        || filters::apply(key, value, negated_key, state)
        || structure::apply(key, value, state)
}
