#[cfg(all(target_os = "macos", feature = "vision-dots-metal"))]
pub(super) fn detect_metal_device_available() -> bool {
    if !metal_probe_enabled() {
        tracing::warn!(
            event = "llm.vision.deepseek.device.metal_probe.disabled",
            requested = "metal",
            fallback = "cpu",
            "DeepSeek Metal probe disabled by XIUXIAN_VISION_METAL_PROBE"
        );
        return false;
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        candle_core::utils::metal_is_available,
    )) {
        Ok(true) => true,
        Ok(false) => {
            tracing::warn!(
                event = "llm.vision.deepseek.device.metal_probe.empty",
                fallback = "cpu",
                "DeepSeek Metal probe found no usable Metal devices"
            );
            false
        }
        Err(_) => {
            tracing::warn!(
                event = "llm.vision.deepseek.device.metal_probe.panic",
                fallback = "cpu",
                "DeepSeek Metal probe panicked; falling back to CPU"
            );
            false
        }
    }
}

#[cfg(not(all(target_os = "macos", feature = "vision-dots-metal")))]
pub(super) fn detect_metal_device_available() -> bool {
    false
}

#[cfg(all(target_os = "macos", feature = "vision-dots-metal"))]
fn metal_probe_enabled() -> bool {
    std::env::var("XIUXIAN_VISION_METAL_PROBE")
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_none_or(|raw| !matches!(raw.as_str(), "0" | "false" | "no" | "off"))
}
