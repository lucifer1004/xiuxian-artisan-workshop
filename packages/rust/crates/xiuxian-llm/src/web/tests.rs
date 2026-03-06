use super::spider::resolve_markdown_content;

#[test]
fn resolve_markdown_content_prefers_cleaned_text() {
    let (content, source) = resolve_markdown_content(
        "normalized text",
        "<html><body>raw</body></html>",
        "https://example.com",
        true,
    );

    assert_eq!(source, "clean_html");
    assert_eq!(content.as_ref(), "normalized text");
}

#[test]
fn resolve_markdown_content_falls_back_to_raw_html() {
    let (content, source) = resolve_markdown_content(
        "",
        "<html><body>raw fallback</body></html>",
        "https://example.com",
        true,
    );

    assert_eq!(source, "raw_html");
    assert!(content.contains("raw fallback"));
}

#[test]
fn resolve_markdown_content_falls_back_to_url_marker() {
    let (content, source) = resolve_markdown_content("", "", "https://example.com", false);

    assert_eq!(source, "url_fallback");
    assert_eq!(content.as_ref(), "https://example.com");
}
