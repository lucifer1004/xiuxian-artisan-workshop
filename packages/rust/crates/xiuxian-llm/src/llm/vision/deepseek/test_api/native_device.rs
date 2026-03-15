//! Native device test API for DeepSeek vision module.

use crate::llm::acceleration::AccelerationDevice;

/// Resolve device kind label with explicit parameters for test assertions.
pub fn resolve_device_kind_label_with_for_tests(
    explicit_device: Option<&str>,
    fallback_mode: AccelerationDevice,
    metal_available: bool,
) -> &'static str {
    super::super::native::resolve_device_kind_label_for_tests(
        explicit_device,
        fallback_mode,
        metal_available,
    )
}

/// Check if quantized snapshot is required for test assertions.
pub fn require_quantized_snapshot_with_for_tests(value: Option<&str>) -> bool {
    super::super::native::require_quantized_snapshot_with_for_tests(value)
}

/// Check snapshot q-offset alignment for test assertions.
pub fn snapshot_qoffset_alignment_with_for_tests(offset: u64, dtype_code: u32) -> bool {
    super::super::native::snapshot_qoffset_alignment_with_for_tests(offset, dtype_code)
}

/// Resolve model kind label with explicit parameters for test assertions.
pub fn resolve_model_kind_label_with_for_tests(raw: Option<&str>) -> &'static str {
    super::super::native::resolve_model_kind_label_with_for_tests(raw)
}

/// Resolve model kind for model root label with explicit parameters for test assertions.
pub fn resolve_model_kind_for_model_root_label_with_for_tests(
    configured_model_kind: Option<&str>,
    model_root: &std::path::Path,
) -> &'static str {
    super::super::native::resolve_model_kind_for_model_root_label_with_for_tests(
        configured_model_kind,
        model_root,
    )
}

/// Check if error indicates CPU fallback should be retried for test assertions.
pub fn should_retry_cpu_fallback_with_for_tests(error_text: &str) -> bool {
    super::super::native::should_retry_with_cpu_fallback_for_tests(error_text)
}
