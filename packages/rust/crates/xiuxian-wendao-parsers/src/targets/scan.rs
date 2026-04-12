use comrak::{
    Arena, Options,
    nodes::{AstNode, NodeValue},
    parse_document,
};

use super::types::{MarkdownTargetOccurrence, MarkdownTargetOccurrenceKind};
use crate::sourcepos::sourcepos_to_byte_range;

pub(super) fn extract_targets_with_comrak(markdown: &str) -> Vec<MarkdownTargetOccurrence> {
    let mut options = Options::default();
    options.extension.wikilinks_title_after_pipe = true;

    let arena = Arena::new();
    let root_node = parse_document(&arena, markdown, &options);

    let mut occurrences = Vec::new();
    for node in root_node.descendants() {
        let sourcepos = node.data().sourcepos;
        let Some(byte_range) = sourcepos_to_byte_range(markdown, sourcepos) else {
            continue;
        };
        let line_range = (sourcepos.start.line.max(1), sourcepos.end.line.max(1));
        let occurrence = match &node.data().value {
            NodeValue::Link(link) => Some(MarkdownTargetOccurrence::new(
                MarkdownTargetOccurrenceKind::MarkdownLink,
                link.url.clone(),
                byte_range,
                line_range,
            )),
            NodeValue::Image(image) => Some(MarkdownTargetOccurrence::new(
                MarkdownTargetOccurrenceKind::MarkdownImage,
                image.url.clone(),
                byte_range,
                line_range,
            )),
            NodeValue::WikiLink(link) => Some(MarkdownTargetOccurrence::new(
                if is_embedded_wikilink(node) {
                    MarkdownTargetOccurrenceKind::WikiEmbed
                } else {
                    MarkdownTargetOccurrenceKind::WikiLink
                },
                link.url.clone(),
                byte_range,
                line_range,
            )),
            _ => None,
        };
        if let Some(occurrence) = occurrence {
            occurrences.push(occurrence);
        }
    }
    occurrences
}

fn is_embedded_wikilink(node: &AstNode<'_>) -> bool {
    let Some(previous) = node.previous_sibling() else {
        return false;
    };
    let NodeValue::Text(text) = &previous.data().value else {
        return false;
    };
    text.as_ref().ends_with('!')
}
