use super::super::references::{
    MarkdownReferenceKind, extract_references, parse_reference_literal,
};
use super::types::MarkdownWikiLink;

/// Extract ordinary body wikilinks from Markdown in document order.
///
/// Embedded wikilinks such as `![[note]]` are skipped so this surface remains
/// aligned with ordinary body-link topology parsing.
#[must_use]
pub fn extract_wikilinks(markdown: &str) -> Vec<MarkdownWikiLink> {
    extract_references(markdown)
        .into_iter()
        .filter(|reference| reference.kind == MarkdownReferenceKind::WikiLink)
        .map(|reference| {
            MarkdownWikiLink::new(
                reference.target,
                reference.target_address,
                reference.original,
            )
        })
        .collect()
}

/// Parse one standalone wikilink literal.
#[must_use]
pub fn parse_wikilink_literal(text: &str) -> Option<MarkdownWikiLink> {
    let parsed = parse_reference_literal(text)?;
    (parsed.kind == MarkdownReferenceKind::WikiLink)
        .then(|| MarkdownWikiLink::new(parsed.target, parsed.target_address, parsed.original))
}
