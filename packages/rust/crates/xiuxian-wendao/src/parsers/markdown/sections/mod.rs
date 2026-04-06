//! Markdown section parsing for link-graph indexing.

mod extract;
mod logbook;
mod properties;
mod types;

pub(in crate::parsers::markdown) use extract::{
    extract_sections, extract_sections_without_source_context,
};
pub use types::{LogbookEntry, ParsedSection};

#[cfg(test)]
pub(crate) use logbook::{extract_logbook_entries, parse_logbook_entry};
#[cfg(test)]
pub(crate) use properties::{extract_property_drawers, parse_property_drawer};

#[cfg(test)]
#[path = "../../../../tests/unit/parsers/markdown/sections.rs"]
mod tests;
