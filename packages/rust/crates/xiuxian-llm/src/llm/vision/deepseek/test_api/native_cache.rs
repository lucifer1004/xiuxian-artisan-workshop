use crate::llm::vision::PreparedVisionImage;

pub fn build_cache_key_with_for_tests(
    model_root: &str,
    prepared: &PreparedVisionImage,
    prompt: &str,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    max_new_tokens: usize,
) -> String {
    super::super::native::build_cache_key(
        model_root,
        prepared,
        prompt,
        base_size,
        image_size,
        crop_mode,
        max_new_tokens,
    )
}

pub fn fingerprint_cache_len_for_tests() -> usize {
    super::super::native::fingerprint_cache_len_for_tests()
}

pub fn fingerprint_cache_clear_for_tests() {
    super::super::native::fingerprint_cache_clear_for_tests();
}

pub fn local_cache_get_for_tests(key: &str) -> Option<String> {
    super::super::native::local_cache_get_for_tests(key)
}

pub fn local_cache_set_with_max_entries_for_tests(key: &str, markdown: &str, max_entries: usize) {
    super::super::native::local_cache_set_with_max_entries_for_tests(key, markdown, max_entries);
}

pub fn local_cache_clear_for_tests() {
    super::super::native::local_cache_clear_for_tests();
}

pub fn valkey_get_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    super::super::native::valkey_get_with_for_tests(
        valkey_url,
        key_prefix,
        ttl_secs,
        io_timeout_ms,
        key,
    )
}

pub fn valkey_set_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    super::super::native::valkey_set_with_for_tests(
        valkey_url,
        key_prefix,
        ttl_secs,
        io_timeout_ms,
        key,
        markdown,
    )
}

pub fn normalize_valkey_timeout_ms_for_tests(io_timeout_ms: u64) -> u64 {
    super::super::native::normalize_valkey_timeout_ms_for_tests(io_timeout_ms)
}

pub fn normalize_cache_text_view_for_tests(text: &str) -> Option<String> {
    super::super::native::normalize_cache_text_view_for_tests(text)
}

pub fn normalize_cache_text_owned_for_tests(text: String) -> Option<String> {
    super::super::native::normalize_cache_text_owned_for_tests(text)
}

pub fn store_markdown_in_cache_for_tests(key: &str, value: &str) {
    super::super::native::store_markdown_in_cache_for_tests(key, value);
}

pub fn cache_layer_labels_for_tests() -> (&'static str, &'static str) {
    super::super::native::cache_layer_labels_for_tests()
}
