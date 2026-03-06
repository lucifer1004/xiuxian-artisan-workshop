use crate::llm::vision::PreparedVisionImage;

pub(crate) fn build_cache_key_with_for_tests(
    model_root: &str,
    prepared: &PreparedVisionImage,
    prompt: &str,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    max_new_tokens: usize,
) -> String {
    super::super::native::DeepseekNativeCacheTestFacade::build_cache_key(
        model_root,
        prepared,
        prompt,
        base_size,
        image_size,
        crop_mode,
        max_new_tokens,
    )
}

pub(crate) fn valkey_get_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    super::super::native::DeepseekNativeCacheTestFacade::valkey_get_with(
        valkey_url,
        key_prefix,
        ttl_secs,
        io_timeout_ms,
        key,
    )
}

pub(crate) fn valkey_set_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    super::super::native::DeepseekNativeCacheTestFacade::valkey_set_with(
        valkey_url,
        key_prefix,
        ttl_secs,
        io_timeout_ms,
        key,
        markdown,
    )
}

pub(crate) fn normalize_valkey_timeout_ms_for_tests(io_timeout_ms: u64) -> u64 {
    super::super::native::DeepseekNativeCacheTestFacade::normalize_valkey_timeout_ms(io_timeout_ms)
}

pub(crate) fn local_cache_get_for_tests(key: &str) -> Option<String> {
    super::super::native::DeepseekNativeCacheTestFacade::local_get(key)
}

pub(crate) fn local_cache_set_with_max_entries_for_tests(
    key: &str,
    markdown: &str,
    max_entries: usize,
) {
    super::super::native::DeepseekNativeCacheTestFacade::local_set_with_max_entries(
        key,
        markdown,
        max_entries,
    );
}

pub(crate) fn local_cache_clear_for_tests() {
    super::super::native::DeepseekNativeCacheTestFacade::local_clear();
}

pub(crate) fn fingerprint_cache_len_for_tests() -> usize {
    super::super::native::DeepseekNativeCacheTestFacade::fingerprint_cache_len()
}

pub(crate) fn fingerprint_cache_clear_for_tests() {
    super::super::native::DeepseekNativeCacheTestFacade::fingerprint_cache_clear();
}

pub(crate) fn normalize_cache_text_view_for_tests(text: &str) -> Option<String> {
    super::super::native::DeepseekNativeCacheTestFacade::normalize_text_view(text)
}

pub(crate) fn normalize_cache_text_owned_for_tests(text: String) -> Option<String> {
    super::super::native::DeepseekNativeCacheTestFacade::normalize_text_owned(text)
}

pub(crate) fn store_markdown_in_cache_for_tests(cache_key: &str, markdown: &str) {
    super::super::native::DeepseekNativeCacheTestFacade::store_markdown(cache_key, markdown);
}

pub(crate) fn cache_layer_labels_for_tests() -> (&'static str, &'static str) {
    super::super::native::DeepseekNativeCacheTestFacade::cache_layer_labels()
}
