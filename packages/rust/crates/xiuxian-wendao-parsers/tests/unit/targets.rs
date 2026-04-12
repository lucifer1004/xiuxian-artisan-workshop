use xiuxian_wendao_parsers::targets::{
    MarkdownTargetOccurrenceKind, TargetOccurrenceCore, extract_targets,
};

#[test]
fn extract_targets_preserves_links_and_images_in_document_order() {
    let markdown = r"
Body [Guide](docs/guide.md#intro) and [[docs/spec.md|Spec]].

![Image](assets/logo.png)
![[IgnoredEmbed]]
[[graph-c]]
[Local](#local-section)
";

    let targets = extract_targets(markdown);
    assert_eq!(targets.len(), 5);

    let first: &TargetOccurrenceCore<MarkdownTargetOccurrenceKind> = &targets[0];
    assert_eq!(first.target, "docs/guide.md#intro");
    assert_eq!(first.line_range, (2, 2));

    assert_eq!(targets[0].kind, MarkdownTargetOccurrenceKind::MarkdownLink);
    assert_eq!(targets[0].target, "docs/guide.md#intro");
    assert_eq!(
        &markdown[targets[0].byte_range.0..targets[0].byte_range.1],
        "[Guide](docs/guide.md#intro)"
    );

    assert_eq!(targets[1].kind, MarkdownTargetOccurrenceKind::WikiLink);
    assert_eq!(targets[1].target, "docs/spec.md");
    assert_eq!(targets[1].line_range, (2, 2));

    assert_eq!(targets[2].kind, MarkdownTargetOccurrenceKind::MarkdownImage);
    assert_eq!(targets[2].target, "assets/logo.png");
    assert_eq!(targets[2].line_range, (4, 4));

    assert_eq!(targets[3].kind, MarkdownTargetOccurrenceKind::WikiLink);
    assert_eq!(targets[3].target, "graph-c");
    assert_eq!(targets[3].line_range, (6, 6));

    assert_eq!(targets[4].kind, MarkdownTargetOccurrenceKind::MarkdownLink);
    assert_eq!(targets[4].target, "#local-section");
    assert_eq!(targets[4].line_range, (7, 7));
}
