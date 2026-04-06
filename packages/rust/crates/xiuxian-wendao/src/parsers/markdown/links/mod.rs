mod api;
mod normalize;
mod parse_target;
mod scan;
mod types;

pub(in crate::parsers::markdown) use api::extract_link_targets;
