use crate::llm::acceleration::AccelerationDevice;
use std::path::Path;

/// Resolve `DeepSeek` OCR execution device label for tests.
#[must_use]
pub fn resolve_deepseek_device_kind_label_for_tests(
    explicit_device: Option<&str>,
    fallback_mode: AccelerationDevice,
    metal_available: bool,
) -> &'static str {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_device_kind_label_with_for_tests(
            explicit_device,
            fallback_mode,
            metal_available,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = explicit_device;
        let _ = fallback_mode;
        let _ = metal_available;
        "cpu"
    }
}

/// Evaluate whether `DeepSeek` OCR errors should trigger CPU fallback retry.
#[must_use]
pub fn should_retry_deepseek_cpu_fallback_for_tests(error_text: &str) -> bool {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::should_retry_cpu_fallback_with_for_tests(error_text)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = error_text;
        false
    }
}

/// Resolve whether `DeepSeek` OCR should require a quantized snapshot.
#[must_use]
pub fn require_quantized_deepseek_snapshot_for_tests(value: Option<&str>) -> bool {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::require_quantized_snapshot_with_for_tests(value)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = value;
        true
    }
}

/// Validate whether a DSQ tensor payload offset is aligned for Candle quantized loading.
#[must_use]
pub fn deepseek_snapshot_qoffset_aligned_for_tests(offset: u64, dtype_code: u32) -> bool {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::snapshot_qoffset_alignment_with_for_tests(offset, dtype_code)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = offset;
        let _ = dtype_code;
        false
    }
}

/// Resolve `DeepSeek` OCR model-kind label after parser normalization.
#[must_use]
pub fn resolve_deepseek_model_kind_label_for_tests(value: Option<&str>) -> &'static str {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_model_kind_label_with_for_tests(value)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = value;
        "deepseek"
    }
}

/// Resolve `DeepSeek` OCR model-kind label after model-root auto-detection.
#[must_use]
pub fn resolve_deepseek_model_kind_for_model_root_label_for_tests(
    value: Option<&str>,
    model_root: &Path,
) -> &'static str {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_model_kind_for_model_root_label_with_for_tests(
            value, model_root,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = value;
        let _ = model_root;
        "deepseek"
    }
}

/// Resolve `DeepSeek` OCR weights path with explicit model kind for tests.
///
/// # Errors
///
/// Returns an error when no compatible weights file is found under `model_root`,
/// or when `override_path` is provided but does not resolve to a valid file.
pub fn resolve_deepseek_weights_path_for_tests(
    model_root: &Path,
    model_kind: Option<&str>,
    override_path: Option<&str>,
) -> Result<String, String> {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_weights_path_with_for_tests(
            model_root,
            model_kind,
            override_path,
        )
        .map(|path| path.display().to_string())
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = model_root;
        let _ = model_kind;
        let _ = override_path;
        Err("vision-dots feature is disabled".to_string())
    }
}
