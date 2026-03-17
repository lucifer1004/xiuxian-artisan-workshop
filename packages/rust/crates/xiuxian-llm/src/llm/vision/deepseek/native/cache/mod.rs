mod key;
mod local;
mod text;
mod valkey;

pub use self::local::{local_clear_for_tests, local_get, local_set};
pub use self::text::{normalize_cache_text_owned_for_tests, normalize_cache_text_view_for_tests};
pub use self::valkey::{
    get_with as valkey_get_with_for_tests, normalize_valkey_timeout_ms_for_tests,
    set_with as valkey_set_with_for_tests,
};

// Re-export common functions for engine module (CLEAN NAMES)
pub use self::valkey::set as valkey_set;

pub fn build_cache_key(
    model_root: &str,
    prepared: &crate::llm::vision::PreparedVisionImage,
    prompt: &str,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    max_new_tokens: usize,
) -> String {
    key::build_cache_key(
        model_root,
        prepared,
        prompt,
        base_size,
        image_size,
        crop_mode,
        max_new_tokens,
    )
}

pub fn local_set_with_max_entries_for_tests(key: &str, markdown: &str, max_entries: usize) {
    local::DeepseekLocalCache::set_with_max_entries(key, markdown, max_entries);
}

pub fn cache_layer_labels_for_tests() -> (&'static str, &'static str) {
    ("local", "valkey")
}
