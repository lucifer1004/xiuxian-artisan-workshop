use std::path::Path;

use xiuxian_wendao_parsers::targets::{MarkdownTargetOccurrence, MarkdownTargetOccurrenceKind};

use super::extract_link_targets_from_occurrences;

#[test]
fn extract_link_targets_from_occurrences_normalizes_markdown_and_wikilink_targets() {
    let occurrences = vec![
        MarkdownTargetOccurrence {
            kind: MarkdownTargetOccurrenceKind::MarkdownLink,
            target: "docs/guide.md#intro".to_string(),
            byte_range: (0, 20),
            line_range: (1, 1),
        },
        MarkdownTargetOccurrence {
            kind: MarkdownTargetOccurrenceKind::MarkdownImage,
            target: "assets/logo.png".to_string(),
            byte_range: (21, 36),
            line_range: (2, 2),
        },
        MarkdownTargetOccurrence {
            kind: MarkdownTargetOccurrenceKind::WikiLink,
            target: "graph-c".to_string(),
            byte_range: (37, 44),
            line_range: (3, 3),
        },
        MarkdownTargetOccurrence {
            kind: MarkdownTargetOccurrenceKind::MarkdownLink,
            target: "#local".to_string(),
            byte_range: (45, 51),
            line_range: (4, 4),
        },
    ];

    let root = Path::new("/tmp/parser-doc");
    let path = Path::new("/tmp/parser-doc/adapter.md");
    let extracted = extract_link_targets_from_occurrences(&occurrences, path, root);

    assert_eq!(extracted.note_links, vec!["docs/guide", "graph-c"]);
    assert_eq!(extracted.attachments, vec!["assets/logo.png"]);
}
