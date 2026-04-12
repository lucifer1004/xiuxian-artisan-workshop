//! Independent parser surfaces and parser-owned contracts for Wendao-adjacent
//! consumers.

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

/// Parser-owned reusable target plus scoped-address contract.
pub mod addressed_target;
/// Shared Markdown block parsing and parser-owned block contracts.
pub mod blocks;
/// Parser-owned Markdown document metadata extraction.
pub mod document;
/// Markdown frontmatter parsing and parser-owned frontmatter contracts.
pub mod frontmatter;
/// Parser-owned source-preserved addressed-target contract.
pub mod literal_addressed_target;
/// Parser-owned Markdown note aggregation.
pub mod note;
/// Parser-owned source-preserved reference payload shared across formats.
pub mod reference_core;
/// Shared Markdown reference parsing and parser-owned link contracts.
pub mod references;
/// Shared Markdown section parsing and parser-owned section contracts.
pub mod sections;
/// Shared source-position helpers used by parser-owned Markdown scans.
pub mod sourcepos;
/// Parser-owned raw Markdown target-occurrence extraction.
pub mod targets;
/// Shared Markdown wikilink parsing built on top of reference parsing.
pub mod wikilinks;

pub use addressed_target::AddressedTarget;
pub use blocks::{
    BlockCore, BlockKindIdentity, MarkdownBlock, MarkdownBlockKind, compute_block_hash,
    extract_blocks, line_col_to_byte_range,
};
pub use document::{
    DocumentCore, DocumentEnvelope, DocumentFormat, MarkdownDocument, parse_markdown_document,
};
pub use frontmatter::{NoteFrontmatter, parse_frontmatter, split_frontmatter};
pub use literal_addressed_target::LiteralAddressedTarget;
pub use note::{MarkdownNote, MarkdownNoteCore, NoteAggregate, NoteCore, parse_markdown_note};
pub use reference_core::ReferenceCore;
pub use references::{
    MarkdownReference, MarkdownReferenceKind, extract_references, parse_reference_literal,
};
pub use sections::{
    LogbookEntry, MarkdownSection, SectionCore, SectionMetadata, SectionScope, extract_sections,
};
pub use targets::{
    MarkdownTargetOccurrence, MarkdownTargetOccurrenceKind, TargetOccurrenceCore, extract_targets,
};
pub use wikilinks::{MarkdownWikiLink, extract_wikilinks, parse_wikilink_literal};
