//! Wendao adapter surface over parser-owned Markdown section-create helpers.

pub use xiuxian_wendao_parsers::section_create::{
    BuildSectionOptions, InsertionInfo, build_new_sections_content_with_options,
    compute_content_hash, find_insertion_point,
};
