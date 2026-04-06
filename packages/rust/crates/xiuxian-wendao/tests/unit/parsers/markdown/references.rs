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
fn extract_references_snapshot_matches_markdown_contract() {
    let markdown = r"
Body [Guide](references/qianji.toml#flow) and [[references/qianji.toml|Manifest]].

Local [Section](#local-section) and [[#Implementation]] should stay address-only.

Image ![Ignore](assets/logo.png) and embed ![[IgnoredEmbed]] should not become ordinary references.
";

    let payload = json!(extract_references(markdown));
    let expected = read_json_snapshot("parser/markdown/references.json");

    assert_eq!(payload, expected);
}

#[test]
fn parse_reference_literal_accepts_markdown_local_address_targets() {
    let parsed = parse_reference_literal("[Section](#local-section)")
        .unwrap_or_else(|| panic!("local address markdown reference should parse"));

    assert_eq!(parsed.kind, MarkdownReferenceKind::Markdown);
    assert_eq!(parsed.target, None);
    assert_eq!(parsed.target_address.as_deref(), Some("#local-section"));
    assert_eq!(parsed.original, "[Section](#local-section)");
}

#[test]
fn parse_reference_literal_rejects_surrounding_text() {
    assert!(parse_reference_literal("prefix [Guide](references/qianji.toml) suffix").is_none());
}
