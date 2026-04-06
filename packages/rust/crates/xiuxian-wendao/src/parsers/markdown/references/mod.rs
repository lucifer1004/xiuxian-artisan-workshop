mod api;
mod scan;
mod types;

pub use api::{extract_references, parse_reference_literal};
pub use types::{MarkdownReference, MarkdownReferenceKind};

#[cfg(test)]
#[path = "../../../../tests/unit/parsers/markdown/references.rs"]
mod tests;
