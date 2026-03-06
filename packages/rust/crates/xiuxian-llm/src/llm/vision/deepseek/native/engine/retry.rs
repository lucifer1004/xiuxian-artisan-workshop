use deepseek_ocr_core::VisionSettings;

pub(super) fn panic_payload_to_string(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(value) = payload.downcast_ref::<&str>() {
        (*value).to_string()
    } else if let Some(value) = payload.downcast_ref::<String>() {
        value.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

pub(super) fn should_retry_with_safe_vision(
    error_text: &str,
    current_vision: VisionSettings,
) -> bool {
    if !error_text.contains("unsupported Qwen2 query length") {
        return false;
    }
    current_vision.base_size != 448 || current_vision.image_size != 448 || !current_vision.crop_mode
}

pub(super) fn safe_vision_settings() -> VisionSettings {
    VisionSettings {
        base_size: 448,
        image_size: 448,
        crop_mode: true,
    }
}

pub(super) fn should_retry_with_cpu_fallback(error_text: &str) -> bool {
    let normalized = error_text.to_ascii_lowercase();
    normalized.contains("metal error")
        && normalized.contains("failed to create metal resource")
        && normalized.contains("buffer")
}
