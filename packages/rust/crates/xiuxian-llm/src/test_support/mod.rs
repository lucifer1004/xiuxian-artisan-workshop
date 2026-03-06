mod acceleration;
mod deepseek_cache;
mod deepseek_cache_api;
mod deepseek_config;
mod deepseek_runtime;

pub use acceleration::{
    load_acceleration_device_with_paths, parse_acceleration_device_for_tests,
    resolve_acceleration_device_with_for_tests,
};
pub use deepseek_cache::{DeepseekCacheKeyInput, DeepseekCacheTestFacade};
pub use deepseek_cache_api::{
    build_deepseek_cache_key_for_tests, build_deepseek_cache_key_from_prepared_for_tests,
    deepseek_cache_layer_labels_for_tests, deepseek_fingerprint_cache_clear_for_tests,
    deepseek_fingerprint_cache_len_for_tests, deepseek_local_cache_clear_for_tests,
    deepseek_local_cache_get_for_tests, deepseek_local_cache_set_with_max_entries_for_tests,
    deepseek_store_markdown_in_cache_for_tests, deepseek_valkey_get_with_for_tests,
    deepseek_valkey_set_with_for_tests, normalize_deepseek_cache_text_owned_for_tests,
    normalize_deepseek_cache_text_view_for_tests, normalize_deepseek_valkey_timeout_ms_for_tests,
};
pub use deepseek_config::{
    DeepseekCacheConfigSnapshot, DeepseekConfigSnapshot, load_deepseek_config_with_paths,
    normalize_deepseek_model_root, resolve_deepseek_model_root_with,
};
pub use deepseek_runtime::{
    deepseek_snapshot_qoffset_aligned_for_tests, require_quantized_deepseek_snapshot_for_tests,
    resolve_deepseek_device_kind_label_for_tests,
    resolve_deepseek_model_kind_for_model_root_label_for_tests,
    resolve_deepseek_model_kind_label_for_tests, resolve_deepseek_weights_path_for_tests,
    should_retry_deepseek_cpu_fallback_for_tests,
};
