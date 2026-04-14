use super::types::{MarkdownNote, MarkdownNoteCore};
use crate::references::extract_references;
use crate::targets::extract_targets;
use crate::toc::parse_markdown_toc;

/// Parse a parser-owned Markdown note aggregate from raw content.
#[must_use]
pub fn parse_markdown_note(content: &str, fallback_title: &str) -> MarkdownNote {
    let toc = parse_markdown_toc(content, fallback_title);
    let document = toc.document;
    let body = document.core.body.as_str();
    let references = extract_references(body);
    let targets = extract_targets(body);

    MarkdownNote {
        document,
        core: MarkdownNoteCore {
            references,
            targets,
            sections: toc.sections,
        },
    }
}
