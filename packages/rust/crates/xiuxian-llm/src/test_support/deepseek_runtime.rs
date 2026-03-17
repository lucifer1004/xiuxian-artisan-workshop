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

/// Resolve the effective `DeepSeek` OCR engine device after fallback selection.
#[must_use]
pub fn resolve_deepseek_engine_device_label_for_tests(
    requested_device: &str,
    force_cpu: bool,
) -> &'static str {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_engine_device_label_with_for_tests(
            requested_device,
            force_cpu,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = requested_device;
        let _ = force_cpu;
        "cpu"
    }
}

/// Resolve the `DeepSeek` OCR model load dtype after backend defaulting.
#[must_use]
pub fn resolve_deepseek_load_dtype_label_for_tests(
    prepared_dtype: Option<&str>,
    accelerated_backend: bool,
) -> &'static str {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_model_load_dtype_label_with_for_tests(
            prepared_dtype,
            accelerated_backend,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = prepared_dtype;
        let _ = accelerated_backend;
        "f32"
    }
}

/// Resolve the effective `DeepSeek` OCR vision settings after defaulting.
#[must_use]
pub fn resolve_deepseek_vision_settings_for_tests(
    base_size: Option<u32>,
    image_size: Option<u32>,
    crop_mode: Option<bool>,
) -> (u32, u32, bool) {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_vision_settings_with_for_tests(
            base_size, image_size, crop_mode,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = base_size;
        let _ = image_size;
        let _ = crop_mode;
        (1_024, 640, true)
    }
}

/// Resolve the low-precision auxiliary preload policy used for `DeepSeek` model loads.
#[must_use]
pub fn resolve_deepseek_low_precision_load_policy_for_tests(
    preload_language_f32_aux: Option<bool>,
    preload_vision_f32_aux: Option<bool>,
    preload_linear_weight_f32: Option<bool>,
    promote_language_input_f32: Option<bool>,
    lazy_moe_experts: Option<bool>,
    lazy_clip_transformer_layers: Option<bool>,
) -> (bool, bool, bool, bool, bool, bool) {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_low_precision_load_policy_with_for_tests(
            preload_language_f32_aux,
            preload_vision_f32_aux,
            preload_linear_weight_f32,
            promote_language_input_f32,
            lazy_moe_experts,
            lazy_clip_transformer_layers,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = preload_language_f32_aux;
        let _ = preload_vision_f32_aux;
        let _ = preload_linear_weight_f32;
        let _ = promote_language_input_f32;
        let _ = lazy_moe_experts;
        let _ = lazy_clip_transformer_layers;
        (true, true, true, true, false, false)
    }
}

/// Resolve the safe fallback `DeepSeek` OCR vision settings used after retry.
#[must_use]
pub fn resolve_deepseek_safe_vision_settings_for_tests() -> (u32, u32, bool) {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::safe_vision_settings_for_tests()
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        (448, 448, true)
    }
}

/// Resolve whether the cached `DeepSeek` OCR engine should be reused.
#[must_use]
pub fn should_reuse_deepseek_engine_cache_for_tests(
    cached_model_root: &str,
    cached_device: &str,
    requested_model_root: &str,
    requested_device: &str,
    force_cpu: bool,
) -> bool {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::should_reuse_engine_cache_with_for_tests(
            cached_model_root,
            cached_device,
            requested_model_root,
            requested_device,
            force_cpu,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = cached_model_root;
        let _ = cached_device;
        let _ = requested_model_root;
        let _ = requested_device;
        let _ = force_cpu;
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
        false
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

/// Resolve `DeepSeek` OCR model-kind label using env/config precedence.
#[must_use]
pub fn resolve_deepseek_model_kind_for_model_root_label_from_sources_for_tests(
    env_value: Option<&str>,
    config_value: Option<&str>,
    model_root: &Path,
) -> &'static str {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_model_kind_for_model_root_label_from_sources_for_tests(
            env_value,
            config_value,
            model_root,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = env_value;
        let _ = config_value;
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

/// Resolve `DeepSeek` OCR snapshot path with an explicit override for tests.
#[must_use]
pub fn resolve_deepseek_snapshot_path_for_tests(
    model_root: &Path,
    override_path: Option<&str>,
) -> Option<String> {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::resolve_snapshot_path_with_for_tests(
            model_root,
            override_path,
        )
        .map(|path| path.display().to_string())
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = model_root;
        let _ = override_path;
        None
    }
}
