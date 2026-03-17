use std::path::{Path, PathBuf};

mod cache;
mod engine;
mod env;

use super::model_kind::VisionModelKind;

pub(super) use engine::infer;
pub(crate) use engine::load_only_for_tests;
pub(super) use engine::prewarm;
pub(crate) use engine::require_quantized_snapshot_with_for_tests;
pub(crate) use engine::reset_engine_state_for_tests;
pub(crate) use engine::resolve_engine_device_label_with_for_tests;
pub(crate) use engine::resolve_low_precision_load_policy_for_tests;
pub(crate) use engine::resolve_model_kind_for_model_root_label_from_sources_for_tests;
pub(crate) use engine::resolve_model_kind_for_model_root_label_with_for_tests;
pub(crate) use engine::resolve_model_kind_label_with_for_tests;
pub(crate) use engine::resolve_model_load_dtype_label_for_tests;
pub(crate) use engine::resolve_vision_settings_with_for_tests;
pub(crate) use engine::safe_vision_settings_for_tests;
pub(crate) use engine::should_reuse_engine_cache_for_tests;
pub(crate) use engine::snapshot_qoffset_alignment_with_for_tests;
pub use engine::{DsqRepairResult, repair_dsq_if_needed};
pub(in crate::llm::vision::deepseek) use env::local_runtime_may_use_metal;
pub(crate) use env::resolve_device_kind_label_for_tests;

// Re-export cache functions for test_api module
pub(in crate::llm::vision::deepseek) use cache::build_cache_key;
pub(in crate::llm::vision::deepseek) use cache::cache_layer_labels_for_tests;
pub(in crate::llm::vision::deepseek) use cache::local_clear_for_tests as local_cache_clear_for_tests;
pub(in crate::llm::vision::deepseek) use cache::local_get as local_cache_get_for_tests;
pub(in crate::llm::vision::deepseek) use cache::local_set_with_max_entries_for_tests as local_cache_set_with_max_entries_for_tests;
pub(in crate::llm::vision::deepseek) use cache::normalize_cache_text_owned_for_tests;
pub(in crate::llm::vision::deepseek) use cache::normalize_cache_text_view_for_tests;
pub(in crate::llm::vision::deepseek) use cache::normalize_valkey_timeout_ms_for_tests;
pub(in crate::llm::vision::deepseek) use cache::valkey_get_with_for_tests;
pub(in crate::llm::vision::deepseek) use cache::valkey_set_with_for_tests;
pub(crate) use engine::should_retry_with_cpu_fallback_for_tests;
pub(in crate::llm::vision::deepseek) use engine::store_markdown_in_cache_for_tests;

pub(in crate::llm::vision::deepseek) fn resolve_snapshot_path_with(
    model_root: &Path,
    override_path: Option<&Path>,
) -> Option<PathBuf> {
    env::resolve_snapshot_path_with_for_tests(model_root, override_path)
}

pub(crate) fn resolve_snapshot_path_with_for_tests(
    model_root: &Path,
    override_path: Option<&Path>,
) -> Option<PathBuf> {
    env::resolve_snapshot_path_with_for_tests(model_root, override_path)
}

pub(crate) fn resolve_weights_path_with_for_tests(
    model_root: &Path,
    model_kind: VisionModelKind,
    override_path: Option<&str>,
) -> Result<PathBuf, String> {
    env::resolve_weights_path_with_for_tests(model_root, model_kind, override_path)
}
