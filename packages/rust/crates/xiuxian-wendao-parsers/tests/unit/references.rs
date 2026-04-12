use xiuxian_wendao_parsers::references::{
    MarkdownReferenceKind, extract_references, parse_reference_literal,
};
use xiuxian_wendao_parsers::{LiteralAddressedTarget, ReferenceCore};

#[test]
fn extract_references_keeps_markdown_and_wikilink_contract() {
    let markdown = r"
Body [Guide](references/qianji.toml#flow) and [[references/qianji.toml|Manifest]].

Local [Section](#local-section) and [[#Implementation]] should stay address-only.

Image ![Ignore](assets/logo.png) and embed ![[IgnoredEmbed]] should not become ordinary references.
";

    let references = extract_references(markdown);
    assert_eq!(references.len(), 4);

    assert_eq!(references[0].kind, MarkdownReferenceKind::Markdown);
    assert_eq!(
        references[0].addressed_target.target.as_deref(),
        Some("references/qianji.toml")
    );
    assert_eq!(
        references[0].addressed_target.target_address.as_deref(),
        Some("#flow")
    );

    assert_eq!(references[1].kind, MarkdownReferenceKind::WikiLink);
    assert_eq!(
        references[1].addressed_target.target.as_deref(),
        Some("references/qianji.toml")
    );
    assert_eq!(references[1].addressed_target.target_address, None);

    assert_eq!(references[2].kind, MarkdownReferenceKind::Markdown);
    assert_eq!(references[2].addressed_target.target, None);
    assert_eq!(
        references[2].addressed_target.target_address.as_deref(),
        Some("#local-section")
    );

    assert_eq!(references[3].kind, MarkdownReferenceKind::WikiLink);
    assert_eq!(references[3].addressed_target.target, None);
    assert_eq!(
        references[3].addressed_target.target_address.as_deref(),
        Some("#Implementation")
    );
}

#[test]
fn parse_reference_literal_accepts_markdown_local_address_targets() {
    let parsed = parse_reference_literal("[Section](#local-section)")
        .unwrap_or_else(|| panic!("local address markdown reference should parse"));

    assert_eq!(parsed.kind, MarkdownReferenceKind::Markdown);
    assert_eq!(parsed.addressed_target.target, None);
    assert_eq!(
        parsed.addressed_target.target_address.as_deref(),
        Some("#local-section")
    );
    assert_eq!(parsed.original, "[Section](#local-section)");
}

#[test]
fn parse_reference_literal_wraps_shared_addressed_target_core() {
    let parsed = parse_reference_literal("[[FactoryPattern#Examples]]")
        .unwrap_or_else(|| panic!("addressed wikilink should parse"));
    let core: &ReferenceCore<MarkdownReferenceKind> = &parsed;
    let literal: &LiteralAddressedTarget = core.as_ref();

    assert_eq!(parsed.kind, MarkdownReferenceKind::WikiLink);
    assert_eq!(
        parsed.addressed_target.target.as_deref(),
        Some("FactoryPattern")
    );
    assert_eq!(
        parsed.addressed_target.target_address.as_deref(),
        Some("#Examples")
    );
    assert_eq!(literal.original, "[[FactoryPattern#Examples]]");
}

#[test]
fn parse_reference_literal_rejects_surrounding_text() {
    assert!(parse_reference_literal("prefix [Guide](references/qianji.toml) suffix").is_none());
}
