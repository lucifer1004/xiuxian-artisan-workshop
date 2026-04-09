use pulldown_cmark::CodeBlockKind;

use super::{escape_html_attr, escape_html_text, normalize_code_fence_language};

#[test]
fn normalize_code_fence_language_filters_unsafe_characters() {
    let normalized = normalize_code_fence_language(CodeBlockKind::Fenced("rust<script>".into()));
    assert_eq!(normalized.as_deref(), Some("rustscript"));
}

#[test]
fn escape_html_helpers_escape_reserved_characters() {
    assert_eq!(escape_html_text("a<&>b"), "a&lt;&amp;&gt;b");
    assert_eq!(escape_html_attr("\"a'&"), "&quot;a&#39;&amp;");
}
