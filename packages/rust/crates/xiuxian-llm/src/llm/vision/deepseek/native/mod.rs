mod cache;
mod engine;
mod env;
mod test_cache;

pub(super) use engine::infer;
pub(super) use engine::prewarm;
pub(crate) use engine::require_quantized_snapshot_with_for_tests;
pub(crate) use engine::resolve_model_kind_for_model_root_label_with_for_tests;
pub(crate) use engine::resolve_model_kind_label_with_for_tests;
pub(crate) use engine::snapshot_qoffset_alignment_with_for_tests;
pub use engine::{DsqRepairResult, repair_dsq_if_needed};
pub(in crate::llm::vision::deepseek) use env::local_runtime_may_use_metal;
pub(crate) use env::resolve_device_kind_label_for_tests;
pub(crate) use env::resolve_weights_path_with_for_tests;

// Re-export cache functions for test_api module
pub use cache::build_cache_key;
pub use cache::cache_layer_labels_for_tests;
pub use cache::fingerprint_cache_clear_for_tests;
pub use cache::fingerprint_cache_len_for_tests;
pub use cache::local_clear_for_tests as local_cache_clear_for_tests;
pub use cache::local_get as local_cache_get_for_tests;
pub use cache::local_set_with_max_entries_for_tests as local_cache_set_with_max_entries_for_tests;
pub use cache::normalize_cache_text_owned_for_tests;
pub use cache::normalize_cache_text_view_for_tests;
pub use cache::normalize_valkey_timeout_ms_for_tests;
pub use cache::valkey_get_with_for_tests;
pub use cache::valkey_set_with_for_tests;
pub use test_cache::store_markdown_in_cache_for_tests;

pub(crate) use engine::should_retry_with_cpu_fallback_for_tests;
