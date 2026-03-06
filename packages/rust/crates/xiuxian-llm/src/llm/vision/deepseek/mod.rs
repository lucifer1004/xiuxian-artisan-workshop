mod config;
mod inference;
mod model_kind;
#[cfg(feature = "vision-dots")]
mod native;
mod preprocess_cache;
mod runtime;
mod test_api;
mod util;

pub use inference::{infer_deepseek_ocr_truth, prewarm_deepseek_ocr};
pub use preprocess_cache::preprocess_image_for_ocr;
pub use runtime::{DeepseekRuntime, get_deepseek_runtime};
#[cfg(feature = "vision-dots")]
pub(crate) use test_api::{
    build_cache_key_with_for_tests, cache_layer_labels_for_tests,
    fingerprint_cache_clear_for_tests, fingerprint_cache_len_for_tests,
    local_cache_clear_for_tests, local_cache_get_for_tests,
    local_cache_set_with_max_entries_for_tests, normalize_cache_text_owned_for_tests,
    normalize_cache_text_view_for_tests, normalize_valkey_timeout_ms_for_tests,
    require_quantized_snapshot_with_for_tests, resolve_device_kind_label_with_for_tests,
    resolve_model_kind_for_model_root_label_with_for_tests,
    resolve_model_kind_label_with_for_tests, resolve_weights_path_with_for_tests,
    should_retry_cpu_fallback_with_for_tests, snapshot_qoffset_alignment_with_for_tests,
    store_markdown_in_cache_for_tests, valkey_get_with_for_tests, valkey_set_with_for_tests,
};
pub(crate) use test_api::{
    load_config_with_paths_for_tests, normalize_model_root_for_tests,
    resolve_model_root_with_for_tests,
};
