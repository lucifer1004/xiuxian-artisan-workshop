use deepseek_ocr_core::runtime::DeviceKind;

use crate::llm::acceleration::AccelerationDevice;

pub(super) fn should_probe_metal(
    explicit_device: Option<&str>,
    fallback_mode: AccelerationDevice,
) -> bool {
    match explicit_device {
        Some(value) => matches!(value.to_ascii_lowercase().as_str(), "metal" | "auto"),
        None => matches!(
            fallback_mode,
            AccelerationDevice::Metal | AccelerationDevice::Auto
        ),
    }
}

pub(super) fn resolve_device_kind_from_inputs(
    explicit_device: Option<&str>,
    fallback_mode: AccelerationDevice,
    metal_available: bool,
) -> DeviceKind {
    if let Some(explicit) = explicit_device {
        let mode = parse_explicit_mode(explicit);
        return mode.map_or(DeviceKind::Cpu, |value| {
            resolve_supported_device_kind(value, metal_available)
        });
    }
    resolve_supported_device_kind(fallback_mode, metal_available)
}

fn parse_explicit_mode(explicit_device: &str) -> Option<AccelerationDevice> {
    match explicit_device.to_ascii_lowercase().as_str() {
        "cuda" => Some(AccelerationDevice::Cuda),
        "metal" => Some(AccelerationDevice::Metal),
        "auto" => Some(AccelerationDevice::Auto),
        "cpu" => Some(AccelerationDevice::Cpu),
        _ => None,
    }
}

fn resolve_supported_device_kind(mode: AccelerationDevice, metal_available: bool) -> DeviceKind {
    match mode {
        AccelerationDevice::Cpu => DeviceKind::Cpu,
        AccelerationDevice::Auto => auto_device_kind_for_platform(metal_available),
        AccelerationDevice::Metal => resolve_metal_device_kind(metal_available),
        AccelerationDevice::Cuda => {
            #[cfg(feature = "vision-dots-cuda")]
            {
                DeviceKind::Cuda
            }
            #[cfg(not(feature = "vision-dots-cuda"))]
            {
                tracing::warn!(
                    event = "llm.vision.deepseek.device.unsupported",
                    requested = "cuda",
                    fallback = "cpu",
                    "DeepSeek CUDA acceleration requested but `vision-dots-cuda` feature is disabled"
                );
                DeviceKind::Cpu
            }
        }
    }
}

fn resolve_metal_device_kind(metal_available: bool) -> DeviceKind {
    #[cfg(feature = "vision-dots-metal")]
    {
        if metal_available {
            return DeviceKind::Metal;
        }
        tracing::warn!(
            event = "llm.vision.deepseek.device.unavailable",
            requested = "metal",
            fallback = "cpu",
            "DeepSeek metal acceleration requested but no Metal device was detected"
        );
        DeviceKind::Cpu
    }
    #[cfg(not(feature = "vision-dots-metal"))]
    {
        let _ = metal_available;
        tracing::warn!(
            event = "llm.vision.deepseek.device.unsupported",
            requested = "metal",
            fallback = "cpu",
            "DeepSeek metal acceleration requested but `vision-dots-metal` feature is disabled"
        );
        DeviceKind::Cpu
    }
}

fn auto_device_kind_for_platform(metal_available: bool) -> DeviceKind {
    #[cfg(all(target_os = "macos", feature = "vision-dots-metal"))]
    {
        if metal_available {
            return DeviceKind::Metal;
        }
        tracing::warn!(
            event = "llm.vision.deepseek.device.auto_fallback",
            requested = "auto",
            fallback = "cpu",
            "DeepSeek auto acceleration selected CPU because no Metal device was detected"
        );
        DeviceKind::Cpu
    }
    #[cfg(all(target_os = "linux", feature = "vision-dots-cuda"))]
    {
        let _ = metal_available;
        DeviceKind::Cuda
    }
    #[cfg(not(any(
        all(target_os = "macos", feature = "vision-dots-metal"),
        all(target_os = "linux", feature = "vision-dots-cuda"),
    )))]
    {
        let _ = metal_available;
        DeviceKind::Cpu
    }
}
