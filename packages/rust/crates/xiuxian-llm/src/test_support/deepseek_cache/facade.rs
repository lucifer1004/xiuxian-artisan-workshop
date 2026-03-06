use crate::llm::vision::PreparedVisionImage;

use super::key::DeepseekCacheKeyInput;
use super::{key, local, text, valkey, write};

/// Consolidated cache-test facade for `DeepSeek` helper operations.
pub struct DeepseekCacheTestFacade;

impl DeepseekCacheTestFacade {
    /// Build `DeepSeek` OCR cache key for test assertions.
    #[must_use]
    pub fn build_cache_key(input: &DeepseekCacheKeyInput<'_>) -> String {
        key::build_cache_key(input)
    }

    /// Build `DeepSeek` OCR cache key from a prepared image for test assertions.
    #[must_use]
    pub fn build_cache_key_from_prepared(
        model_root: &str,
        prepared: &PreparedVisionImage,
        prompt: &str,
        base_size: u32,
        image_size: u32,
        crop_mode: bool,
        max_new_tokens: usize,
    ) -> String {
        key::build_cache_key_from_prepared(
            model_root,
            prepared,
            prompt,
            base_size,
            image_size,
            crop_mode,
            max_new_tokens,
        )
    }

    /// Evaluate `DeepSeek` Valkey GET path with explicit cache settings for tests.
    #[must_use]
    pub fn valkey_get_with(
        valkey_url: &str,
        key_prefix: &str,
        ttl_secs: u64,
        io_timeout_ms: u64,
        key: &str,
    ) -> Option<String> {
        valkey::get_with(valkey_url, key_prefix, ttl_secs, io_timeout_ms, key)
    }

    /// Evaluate `DeepSeek` Valkey SET path with explicit cache settings for tests.
    #[must_use]
    pub fn valkey_set_with(
        valkey_url: &str,
        key_prefix: &str,
        ttl_secs: u64,
        io_timeout_ms: u64,
        key: &str,
        markdown: &str,
    ) -> bool {
        valkey::set_with(
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
    pub fn normalize_valkey_timeout_ms(io_timeout_ms: u64) -> u64 {
        valkey::normalize_timeout_ms(io_timeout_ms)
    }

    /// Read a `DeepSeek` local cache entry by key for test assertions.
    #[must_use]
    pub fn local_get(key: &str) -> Option<String> {
        local::get(key)
    }

    /// Write a `DeepSeek` local cache entry with explicit cache-cap settings.
    pub fn local_set_with_max_entries(key: &str, markdown: &str, max_entries: usize) {
        local::set_with_max_entries(key, markdown, max_entries);
    }

    /// Clear all `DeepSeek` local cache entries for deterministic tests.
    pub fn local_clear() {
        local::clear();
    }

    /// Return the number of prepared-image fingerprint cache entries.
    #[must_use]
    pub fn fingerprint_cache_len() -> usize {
        #[cfg(feature = "vision-dots")]
        {
            crate::llm::vision::deepseek::fingerprint_cache_len_for_tests()
        }
        #[cfg(not(feature = "vision-dots"))]
        {
            0
        }
    }

    /// Clear prepared-image fingerprint cache entries for deterministic tests.
    pub fn fingerprint_cache_clear() {
        #[cfg(feature = "vision-dots")]
        {
            crate::llm::vision::deepseek::fingerprint_cache_clear_for_tests();
        }
    }

    /// Execute deepseek cache write pipeline (local + optional valkey) for tests.
    pub fn store_markdown(cache_key: &str, markdown: &str) {
        write::store_markdown(cache_key, markdown);
    }

    /// Return deepseek cache-layer telemetry labels in enum-variant order.
    #[must_use]
    pub fn cache_layer_labels() -> (&'static str, &'static str) {
        write::cache_layer_labels()
    }

    /// Normalize cache text view (`&str`) using deepseek cache read semantics.
    #[must_use]
    pub fn normalize_text_view(text: &str) -> Option<String> {
        text::normalize_view(text)
    }

    /// Normalize owned cache text (`String`) using deepseek cache read semantics.
    #[must_use]
    pub fn normalize_text_owned(text: String) -> Option<String> {
        text::normalize_owned(text)
    }
}
