//! Native device test API for `DeepSeek` vision module.

use crate::llm::acceleration::AccelerationDevice;
use crate::llm::vision::deepseek::native;

/// Resolve device kind label with explicit parameters for test assertions.
#[must_use]
pub fn resolve_device_kind_label_with_for_tests(
    explicit_device: Option<&str>,
    fallback_mode: AccelerationDevice,
    metal_available: bool,
) -> &'static str {
    native::resolve_device_kind_label_for_tests(explicit_device, fallback_mode, metal_available)
}

/// Check if quantized snapshot is required for test assertions.
#[must_use]
pub fn require_quantized_snapshot_with_for_tests(value: Option<&str>) -> bool {
    native::require_quantized_snapshot_with_for_tests(value)
}

/// Check snapshot q-offset alignment for test assertions.
#[must_use]
pub fn snapshot_qoffset_alignment_with_for_tests(offset: u64, dtype_code: u32) -> bool {
    native::snapshot_qoffset_alignment_with_for_tests(offset, dtype_code)
}

/// Resolve model kind label with explicit parameters for test assertions.
#[must_use]
pub fn resolve_model_kind_label_with_for_tests(raw: Option<&str>) -> &'static str {
    native::resolve_model_kind_label_with_for_tests(raw)
}

/// Resolve model kind for model root label with explicit parameters for test assertions.
#[must_use]
pub fn resolve_model_kind_for_model_root_label_with_for_tests(
    configured_model_kind: Option<&str>,
    model_root: &std::path::Path,
) -> &'static str {
    native::resolve_model_kind_for_model_root_label_with_for_tests(
        configured_model_kind,
        model_root,
    )
}

/// Check if error indicates CPU fallback should be retried for test assertions.
#[must_use]
pub fn should_retry_cpu_fallback_with_for_tests(error_text: &str) -> bool {
    native::should_retry_with_cpu_fallback_for_tests(error_text)
}

/// Resolve the effective engine device label after CPU fallback forcing.
#[must_use]
pub fn resolve_engine_device_label_with_for_tests(
    requested_device: &str,
    force_cpu: bool,
) -> &'static str {
    native::resolve_engine_device_label_with_for_tests(requested_device, force_cpu)
}

/// Resolve the model load dtype label after backend defaulting.
#[must_use]
pub fn resolve_model_load_dtype_label_with_for_tests(
    prepared_dtype: Option<&str>,
    accelerated_backend: bool,
) -> &'static str {
    native::resolve_model_load_dtype_label_for_tests(prepared_dtype, accelerated_backend)
}

/// Resolve the effective vision settings after applying default policy.
#[must_use]
pub fn resolve_vision_settings_with_for_tests(
    base_size: Option<u32>,
    image_size: Option<u32>,
    crop_mode: Option<bool>,
) -> (u32, u32, bool) {
    native::resolve_vision_settings_with_for_tests(base_size, image_size, crop_mode)
}

/// Resolve the safe fallback vision settings used after decode retry escalation.
#[must_use]
pub fn safe_vision_settings_for_tests() -> (u32, u32, bool) {
    native::safe_vision_settings_for_tests()
}

/// Check whether the cached engine should be reused for the current request.
#[must_use]
pub fn should_reuse_engine_cache_with_for_tests(
    cached_model_root: &str,
    cached_device: &str,
    requested_model_root: &str,
    requested_device: &str,
    force_cpu: bool,
) -> bool {
    native::should_reuse_engine_cache_for_tests(
        cached_model_root,
        cached_device,
        requested_model_root,
        requested_device,
        force_cpu,
    )
}
