use super::super::super::preprocess::PreparedVisionImage;
use super::{cache, engine};

pub(in crate::llm::vision::deepseek) struct DeepseekNativeCacheTestFacade;

impl DeepseekNativeCacheTestFacade {
    pub(in crate::llm::vision::deepseek) fn build_cache_key(
        model_root: &str,
        prepared: &PreparedVisionImage,
        prompt: &str,
        base_size: u32,
        image_size: u32,
        crop_mode: bool,
        max_new_tokens: usize,
    ) -> String {
        cache::build_cache_key(
            model_root,
            prepared,
            prompt,
            base_size,
            image_size,
            crop_mode,
            max_new_tokens,
        )
    }

    pub(in crate::llm::vision::deepseek) fn valkey_get_with(
        valkey_url: &str,
        key_prefix: &str,
        ttl_secs: u64,
        io_timeout_ms: u64,
        key: &str,
    ) -> Option<String> {
        cache::valkey_get_with_for_tests(valkey_url, key_prefix, ttl_secs, io_timeout_ms, key)
    }

    pub(in crate::llm::vision::deepseek) fn valkey_set_with(
        valkey_url: &str,
        key_prefix: &str,
        ttl_secs: u64,
        io_timeout_ms: u64,
        key: &str,
        markdown: &str,
    ) -> bool {
        cache::valkey_set_with_for_tests(
            valkey_url,
            key_prefix,
            ttl_secs,
            io_timeout_ms,
            key,
            markdown,
        )
    }

    pub(in crate::llm::vision::deepseek) fn normalize_valkey_timeout_ms(io_timeout_ms: u64) -> u64 {
        cache::normalize_valkey_timeout_ms_for_tests(io_timeout_ms)
    }

    pub(in crate::llm::vision::deepseek) fn local_get(key: &str) -> Option<String> {
        cache::local_get(key)
    }

    pub(in crate::llm::vision::deepseek) fn local_set_with_max_entries(
        key: &str,
        markdown: &str,
        max_entries: usize,
    ) {
        cache::local_set_with_max_entries_for_tests(key, markdown, max_entries);
    }

    pub(in crate::llm::vision::deepseek) fn local_clear() {
        cache::local_clear_for_tests();
    }

    pub(in crate::llm::vision::deepseek) fn fingerprint_cache_len() -> usize {
        cache::fingerprint_cache_len_for_tests()
    }

    pub(in crate::llm::vision::deepseek) fn fingerprint_cache_clear() {
        cache::fingerprint_cache_clear_for_tests();
    }

    pub(in crate::llm::vision::deepseek) fn normalize_text_view(text: &str) -> Option<String> {
        cache::normalize_cache_text_view_for_tests(text)
    }

    pub(in crate::llm::vision::deepseek) fn normalize_text_owned(text: String) -> Option<String> {
        cache::normalize_cache_text_owned_for_tests(text)
    }

    pub(in crate::llm::vision::deepseek) fn store_markdown(cache_key: &str, markdown: &str) {
        engine::store_markdown_in_cache_for_tests(cache_key, markdown);
    }

    pub(in crate::llm::vision::deepseek) fn cache_layer_labels() -> (&'static str, &'static str) {
        engine::cache_layer_labels_for_tests()
    }
}
