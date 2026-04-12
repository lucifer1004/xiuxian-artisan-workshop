use super::parse_target::{parse_markdown_target, parse_wikilink_target};
use super::types::{ExtractedLinkTargets, ParsedTarget};
use std::path::Path;
use xiuxian_wendao_parsers::targets::{MarkdownTargetOccurrence, MarkdownTargetOccurrenceKind};

pub(in crate::parsers::markdown) fn extract_link_targets_from_occurrences(
    occurrences: &[MarkdownTargetOccurrence],
    source_path: &Path,
    root: &Path,
) -> ExtractedLinkTargets {
    extract_link_targets_from_occurrences_in_range(occurrences, source_path, root, None)
}

pub(in crate::parsers::markdown) fn extract_link_targets_from_occurrences_in_range(
    occurrences: &[MarkdownTargetOccurrence],
    source_path: &Path,
    root: &Path,
    byte_range: Option<(usize, usize)>,
) -> ExtractedLinkTargets {
    let mut notes = Vec::new();
    let mut attachments = Vec::new();

    for occurrence in occurrences {
        if let Some((start, end)) = byte_range
            && (occurrence.byte_range.0 < start || occurrence.byte_range.1 > end)
        {
            continue;
        }
        let parsed_target = match occurrence.kind {
            MarkdownTargetOccurrenceKind::MarkdownLink => {
                parse_markdown_target(&occurrence.target, source_path, root)
            }
            MarkdownTargetOccurrenceKind::MarkdownImage => {
                parse_markdown_target(&occurrence.target, source_path, root).and_then(|target| {
                    match target {
                        ParsedTarget::Attachment(path) => Some(ParsedTarget::Attachment(path)),
                        ParsedTarget::Note(_) => None,
                    }
                })
            }
            MarkdownTargetOccurrenceKind::WikiLink => {
                parse_wikilink_target(&occurrence.target, source_path, root, false)
            }
            MarkdownTargetOccurrenceKind::WikiEmbed => {
                parse_wikilink_target(&occurrence.target, source_path, root, true)
            }
        };
        let Some(parsed_target) = parsed_target else {
            continue;
        };
        match parsed_target {
            ParsedTarget::Note(path) => notes.push(path),
            ParsedTarget::Attachment(path) => attachments.push(path),
        }
    }

    notes.sort();
    notes.dedup();
    attachments.sort();
    attachments.dedup();

    ExtractedLinkTargets {
        note_links: notes,
        attachments,
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/parsers/markdown/links/api.rs"]
mod tests;
