mod api;
mod normalize;
mod parse_target;
mod types;

pub(in crate::parsers::markdown) use api::{
    extract_link_targets_from_occurrences, extract_link_targets_from_occurrences_in_range,
};
