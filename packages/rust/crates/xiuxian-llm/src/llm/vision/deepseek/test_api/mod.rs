mod config_runtime;
#[cfg(feature = "vision-dots")]
mod native_cache;
#[cfg(feature = "vision-dots")]
mod native_device;
#[cfg(feature = "vision-dots")]
mod native_paths;

pub(crate) use self::config_runtime::load_config_with_paths_for_tests;
pub use self::config_runtime::{normalize_model_root_for_tests, resolve_model_root_with_for_tests};

#[cfg(feature = "vision-dots")]
pub use self::native_cache::{
    build_cache_key_with_for_tests, cache_layer_labels_for_tests, local_cache_clear_for_tests,
    local_cache_get_for_tests, local_cache_set_with_max_entries_for_tests,
    normalize_cache_text_owned_for_tests, normalize_cache_text_view_for_tests,
    normalize_valkey_timeout_ms_for_tests, store_markdown_in_cache_for_tests,
    valkey_get_with_for_tests, valkey_set_with_for_tests,
};

#[cfg(feature = "vision-dots")]
pub use self::native_device::{
    require_quantized_snapshot_with_for_tests, resolve_device_kind_label_with_for_tests,
    resolve_model_kind_for_model_root_label_with_for_tests,
    resolve_model_kind_label_with_for_tests, should_retry_cpu_fallback_with_for_tests,
    snapshot_qoffset_alignment_with_for_tests,
};

#[cfg(feature = "vision-dots")]
pub use self::native_paths::{
    DsqRepairResult, repair_dsq_if_needed, resolve_weights_path_with_for_tests,
};
