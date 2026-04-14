use super::types::{MarkdownTocDocument, TocDocument};
use crate::document::parse_markdown_document;
use crate::sections::extract_sections;

/// Parse one parser-owned Markdown TOC surface from raw content.
#[must_use]
pub fn parse_markdown_toc(content: &str, fallback_title: &str) -> MarkdownTocDocument {
    let document = parse_markdown_document(content, fallback_title);
    let sections = extract_sections(document.core.body.as_str());
    TocDocument { document, sections }
}
