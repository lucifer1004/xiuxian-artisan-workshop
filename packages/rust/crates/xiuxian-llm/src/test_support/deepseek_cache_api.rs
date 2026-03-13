use super::deepseek_cache::{DeepseekCacheKeyInput, DeepseekCacheTestFacade};
use crate::llm::vision::PreparedVisionImage;
use crate::llm::vision::deepseek::{
    build_cache_key_with_for_tests, fingerprint_cache_clear_for_tests,
    fingerprint_cache_len_for_tests,
};

/// Build `DeepSeek` OCR cache key for test assertions.
#[must_use]
pub fn build_deepseek_cache_key_for_tests(input: &DeepseekCacheKeyInput<'_>) -> String {
    DeepseekCacheTestFacade::build_cache_key(input)
}

/// Build `DeepSeek` OCR cache key from prepared image for test assertions.
#[must_use]
pub fn build_deepseek_cache_key_from_prepared_for_tests(
    model_root: &str,
    prepared: &PreparedVisionImage,
    prompt: &str,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    max_new_tokens: u32,
) -> String {
    build_cache_key_with_for_tests(
        model_root,
        prepared,
        prompt,
        base_size,
        image_size,
        crop_mode,
        usize::try_from(max_new_tokens).unwrap_or(usize::MAX),
    )
}

/// Clear all `DeepSeek` fingerprint cache entries for deterministic tests.
pub fn deepseek_fingerprint_cache_clear_for_tests() {
    fingerprint_cache_clear_for_tests();
}

/// Return the number of entries in the `DeepSeek` fingerprint cache for test assertions.
#[must_use]
pub fn deepseek_fingerprint_cache_len_for_tests() -> usize {
    fingerprint_cache_len_for_tests()
}

/// Evaluate `DeepSeek` Valkey GET path with explicit cache settings for tests.
#[must_use]
pub fn deepseek_valkey_get_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    DeepseekCacheTestFacade::valkey_get_with(valkey_url, key_prefix, ttl_secs, io_timeout_ms, key)
}

/// Evaluate `DeepSeek` Valkey SET path with explicit cache settings for tests.
#[must_use]
pub fn deepseek_valkey_set_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    DeepseekCacheTestFacade::valkey_set_with(
        valkey_url,
        key_prefix,
        ttl_secs,
        io_timeout_ms,
        key,
        markdown,
    )
}

/// Normalize `DeepSeek` Valkey IO timeout for tests.
#[must_use]
pub fn normalize_deepseek_valkey_timeout_ms_for_tests(io_timeout_ms: u64) -> u64 {
    DeepseekCacheTestFacade::normalize_valkey_timeout_ms(io_timeout_ms)
}

/// Read a `DeepSeek` local cache entry by key for test assertions.
#[must_use]
pub fn deepseek_local_cache_get_for_tests(key: &str) -> Option<String> {
    DeepseekCacheTestFacade::local_get(key)
}

/// Write a `DeepSeek` local cache entry with explicit cache-cap settings.
pub fn deepseek_local_cache_set_with_max_entries_for_tests(
    key: &str,
    markdown: &str,
    max_entries: usize,
) {
    DeepseekCacheTestFacade::local_set_with_max_entries(key, markdown, max_entries);
}

/// Clear all `DeepSeek` local cache entries for deterministic tests.
pub fn deepseek_local_cache_clear_for_tests() {
    DeepseekCacheTestFacade::local_clear();
}

/// Execute deepseek cache write pipeline (local + optional valkey) for tests.
pub fn deepseek_store_markdown_in_cache_for_tests(cache_key: &str, markdown: &str) {
    DeepseekCacheTestFacade::store_markdown(cache_key, markdown);
}

/// Return deepseek cache-layer telemetry labels in enum-variant order.
#[must_use]
pub fn deepseek_cache_layer_labels_for_tests() -> (&'static str, &'static str) {
    DeepseekCacheTestFacade::cache_layer_labels()
}

/// Normalize cache text view (`&str`) using deepseek cache read semantics.
#[must_use]
pub fn normalize_deepseek_cache_text_view_for_tests(text: &str) -> Option<String> {
    DeepseekCacheTestFacade::normalize_text_view(text)
}

/// Normalize owned cache text (`String`) using deepseek cache read semantics.
#[must_use]
pub fn normalize_deepseek_cache_text_owned_for_tests(text: String) -> Option<String> {
    DeepseekCacheTestFacade::normalize_text_owned(text)
}
