//! `LinkGraph` reference extraction primitives.
//!
//! Provides `LinkGraph`-oriented consumers over the canonical Markdown
//! wikilink parser in `xiuxian_wendao_parsers::wikilinks`.
//!
//! Cross-note patterns include `[[EntityName]]`, `[[EntityName#Heading]]`,
//! and `[[EntityName#^block-id]]`.

mod extract;
mod model;
mod stats;

pub use extract::{
    count_entity_refs, extract_entity_refs, extract_entity_refs_batch,
    find_notes_referencing_entity, is_valid_entity_ref, parse_entity_ref,
};
pub use model::LinkGraphEntityRef;
pub use stats::{LinkGraphRefStats, get_ref_stats};

#[cfg(test)]
#[path = "../../tests/unit/link_graph_refs.rs"]
mod tests;
