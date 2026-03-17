use super::*;

#[test]
fn json_to_xml_basic() {
    let value = serde_json::json!({
        "name": "alpha",
        "count": 2,
        "active": true,
        "items": [1, "two", null]
    });
    let xml = json_to_xml(&value);
    assert!(xml.contains(r#"<document type="json">"#));
    assert!(xml.contains(r#"<field name="name">"#));
    assert!(xml.contains("<string>alpha</string>"));
    assert!(xml.contains(r#"<number type="integer">2</number>"#));
    assert!(xml.contains("<boolean>true</boolean>"));
    assert!(xml.contains("<null/>"));
}

#[test]
fn json_to_xml_escapes_text() {
    let value = serde_json::json!({ "note": "a < b & c" });
    let xml = json_to_xml(&value);
    assert!(xml.contains("&lt;"));
    assert!(xml.contains("&amp;"));
}

#[test]
fn markdown_to_xml_basic() {
    let xml = markdown_to_xml("# Title\n\nHello **world**");
    assert!(xml.contains(r#"<document type="markdown">"#));
    assert!(xml.contains(r#"<heading level="1">"#));
    assert!(xml.contains("<text>Title</text>"));
    assert!(xml.contains("<paragraph>"));
    assert!(xml.contains("<strong>"));
    assert!(xml.contains("<text>world</text>"));
}
