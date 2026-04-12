use super::types::{MarkdownNote, MarkdownNoteCore};
use crate::document::parse_markdown_document;
use crate::references::extract_references;
use crate::sections::extract_sections;
use crate::targets::extract_targets;

/// Parse a parser-owned Markdown note aggregate from raw content.
#[must_use]
pub fn parse_markdown_note(content: &str, fallback_title: &str) -> MarkdownNote {
    let document = parse_markdown_document(content, fallback_title);
    let body = document.core.body.as_str();
    let references = extract_references(body);
    let targets = extract_targets(body);
    let sections = extract_sections(body);

    MarkdownNote {
        document,
        core: MarkdownNoteCore {
            references,
            targets,
            sections,
        },
    }
}
