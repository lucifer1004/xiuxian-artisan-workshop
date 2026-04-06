use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;

fn read_json_snapshot(relative: &str) -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(relative);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read snapshot {}: {error}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|error| panic!("failed to parse snapshot {}: {error}", path.display()))
}

#[test]
fn extract_wikilinks_snapshot_matches_obsidian_contract() {
    let markdown = r"
Body [[FactoryPattern|FP]] and [[SingletonPattern#Examples]].

Local section [[#Implementation]] should stay address-only.

Embedded ![[IgnoredEmbed]] should not become an ordinary body wikilink.
";

    let payload = json!(extract_wikilinks(markdown));
    let expected = read_json_snapshot("parser/markdown/wikilinks.json");

    assert_eq!(payload, expected);
}

#[test]
fn parse_wikilink_literal_accepts_local_address_targets() {
    let parsed = parse_wikilink_literal("[[#Implementation]]")
        .unwrap_or_else(|| panic!("local address wikilink should parse"));

    assert_eq!(parsed.target, None);
    assert_eq!(parsed.target_address.as_deref(), Some("#Implementation"));
    assert_eq!(parsed.original, "[[#Implementation]]");
}

#[test]
fn parse_wikilink_literal_rejects_surrounding_text() {
    assert!(parse_wikilink_literal("prefix [[FactoryPattern]] suffix").is_none());
}
