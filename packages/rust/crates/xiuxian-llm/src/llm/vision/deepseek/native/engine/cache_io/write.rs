use super::super::super::cache::{local_set, valkey_set};

pub(in crate::llm::vision::deepseek::native::engine) fn store_markdown_in_cache(
    cache_key: &str,
    markdown: &str,
) {
    if markdown.is_empty() {
        tracing::debug!(
            event = "llm.vision.deepseek.cache.store_skipped",
            reason = "empty_markdown",
            "DeepSeek OCR produced empty markdown; skipping cache store"
        );
        return;
    }

    local_set(cache_key, markdown);
    valkey_set(cache_key, markdown);
}

pub(in crate::llm::vision::deepseek::native::engine) fn non_empty_markdown(
    markdown: String,
) -> Option<String> {
    if markdown.is_empty() {
        None
    } else {
        Some(markdown)
    }
}

pub(in crate::llm::vision::deepseek::native) fn store_markdown_in_cache_for_tests(
    cache_key: &str,
    markdown: &str,
) {
    store_markdown_in_cache(cache_key, markdown);
}
