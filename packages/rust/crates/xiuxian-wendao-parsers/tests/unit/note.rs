use xiuxian_wendao_parsers::document::MarkdownDocument;
use xiuxian_wendao_parsers::note::{NoteAggregate, parse_markdown_note};
use xiuxian_wendao_parsers::references::MarkdownReference;
use xiuxian_wendao_parsers::sections::MarkdownSection;
use xiuxian_wendao_parsers::targets::MarkdownTargetOccurrence;

#[test]
fn parse_markdown_note_aggregates_document_sections_and_references() {
    let content = concat!(
        "---\n",
        "title: Aggregate Contract\n",
        "tags:\n",
        "  - parser\n",
        "---\n",
        "\n",
        "Body [Guide](docs/guide.md#intro).\n",
        "\n",
        "# Implementation\n",
        ":PROPERTIES:\n",
        ":ID: impl\n",
        ":END:\n",
        "\n",
        "See [[docs/spec.md|Spec]].\n",
    );

    let note = parse_markdown_note(content, "fallback");

    assert_eq!(note.document.core.title, "Aggregate Contract");
    assert_eq!(note.document.core.tags, vec!["parser"]);
    assert_eq!(note.core.references.len(), 2);
    assert_eq!(note.core.targets.len(), 2);
    assert_eq!(note.core.sections.len(), 2);
    assert_eq!(note.core.sections[1].scope.heading_title, "Implementation");
    assert_eq!(
        note.core.sections[1]
            .metadata
            .attributes
            .get("ID")
            .map(String::as_str),
        Some("impl")
    );
}

#[test]
fn parse_markdown_note_keeps_markdown_link_targets_with_heading_bodies() {
    let content = concat!(
        "# Target Contract\n\n",
        "[Guide](docs/guide.md#intro)\n",
        "![Image](assets/logo.png)\n",
        "[[graph-c]]\n",
    );

    let note = parse_markdown_note(content, "fallback");

    assert_eq!(
        note.document.core.body,
        "# Target Contract\n\n[Guide](docs/guide.md#intro)\n![Image](assets/logo.png)\n[[graph-c]]\n"
    );
    assert_eq!(note.core.targets.len(), 3);
    assert_eq!(note.core.targets[0].target, "docs/guide.md#intro");
    assert_eq!(note.core.targets[1].target, "assets/logo.png");
    assert_eq!(note.core.targets[2].target, "graph-c");
}

#[test]
fn parse_markdown_note_wraps_markdown_items_in_shared_note_core() {
    let note = parse_markdown_note("[Doc](docs/guide.md)\n\n# H\n", "fallback");
    let aggregate: &NoteAggregate<
        MarkdownDocument,
        MarkdownReference,
        MarkdownTargetOccurrence,
        MarkdownSection,
    > = &note;

    assert_eq!(note.core.references.len(), 1);
    assert_eq!(note.core.targets.len(), 1);
    assert_eq!(note.core.sections.len(), 2);
    assert_eq!(aggregate.document.core.title, "H");
}
