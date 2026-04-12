use xiuxian_wendao_parsers::{
    LiteralAddressedTarget,
    wikilinks::{extract_wikilinks, parse_wikilink_literal},
};

#[test]
fn extract_wikilinks_skips_embeds_and_keeps_addresses() {
    let markdown = r"
Body [[FactoryPattern|FP]] and [[SingletonPattern#Examples]].

Local section [[#Implementation]] should stay address-only.

Embedded ![[IgnoredEmbed]] should not become an ordinary body wikilink.
";

    let wikilinks = extract_wikilinks(markdown);
    assert_eq!(wikilinks.len(), 3);

    let first: &LiteralAddressedTarget = &wikilinks[0];
    assert_eq!(first.original, "[[FactoryPattern|FP]]");

    assert_eq!(
        wikilinks[0].addressed_target.target.as_deref(),
        Some("FactoryPattern")
    );
    assert_eq!(wikilinks[0].addressed_target.target_address, None);

    assert_eq!(
        wikilinks[1].addressed_target.target.as_deref(),
        Some("SingletonPattern")
    );
    assert_eq!(
        wikilinks[1].addressed_target.target_address.as_deref(),
        Some("#Examples")
    );

    assert_eq!(wikilinks[2].addressed_target.target, None);
    assert_eq!(
        wikilinks[2].addressed_target.target_address.as_deref(),
        Some("#Implementation")
    );
}

#[test]
fn parse_wikilink_literal_accepts_local_address_targets() {
    let parsed = parse_wikilink_literal("[[#Implementation]]")
        .unwrap_or_else(|| panic!("local address wikilink should parse"));

    assert_eq!(parsed.addressed_target.target, None);
    assert_eq!(
        parsed.addressed_target.target_address.as_deref(),
        Some("#Implementation")
    );
    assert_eq!(parsed.original, "[[#Implementation]]");
}

#[test]
fn parse_wikilink_literal_wraps_shared_addressed_target_core() {
    let parsed = parse_wikilink_literal("[[FactoryPattern#Examples]]")
        .unwrap_or_else(|| panic!("addressed wikilink should parse"));

    assert_eq!(
        parsed.addressed_target.target.as_deref(),
        Some("FactoryPattern")
    );
    assert_eq!(
        parsed.addressed_target.target_address.as_deref(),
        Some("#Examples")
    );
}

#[test]
fn parse_wikilink_literal_rejects_surrounding_text() {
    assert!(parse_wikilink_literal("prefix [[FactoryPattern]] suffix").is_none());
}
