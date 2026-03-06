pub(super) fn store_markdown(cache_key: &str, markdown: &str) {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::store_markdown_in_cache_for_tests(cache_key, markdown);
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = cache_key;
        let _ = markdown;
    }
}

#[must_use]
pub(super) fn cache_layer_labels() -> (&'static str, &'static str) {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::cache_layer_labels_for_tests()
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        ("local", "valkey")
    }
}
