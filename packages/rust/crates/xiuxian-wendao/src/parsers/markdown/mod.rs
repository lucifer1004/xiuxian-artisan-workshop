//! Core markdown note parsing for Wendao document consumers.

#[path = "api.rs"]
mod api;
pub mod blocks;
pub mod code_observation;
mod content;
mod frontmatter;
mod links;
mod paths;
mod relations;
mod sections;
mod time;
mod types;

pub use self::api::parse_note;
pub use self::blocks::extract_blocks;
pub use self::code_observation::{CodeObservation, extract_observations};
pub use self::frontmatter::{NoteFrontmatter, parse_frontmatter};
pub use self::paths::{is_supported_note, normalize_alias};
pub use self::relations::{
    ExplicitRelationSource, ExplicitRelationTarget, ExplicitSectionRelation,
    extract_property_relations, parse_property_relations,
};
pub use self::sections::{LogbookEntry, ParsedSection};
pub use self::types::ParsedNote;

#[cfg(test)]
#[path = "../../../tests/unit/parsers/markdown/namespace.rs"]
mod namespace_tests;
