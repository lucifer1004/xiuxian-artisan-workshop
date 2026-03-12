use deepseek_ocr_core::runtime::DeviceKind;

use super::super::super::config;
use super::super::super::util::non_empty_env;
use crate::llm::acceleration::{AccelerationDevice, resolve_acceleration_device};

mod policy;
mod probe;

pub(in crate::llm::vision::deepseek::native) fn parse_device_kind() -> DeviceKind {
    let explicit_device = non_empty_env("XIUXIAN_VISION_DEVICE").or_else(config::device);
    let fallback_mode = resolve_acceleration_device(None);
    let should_probe_metal = policy::should_probe_metal(explicit_device.as_deref(), fallback_mode);
    let metal_available = if should_probe_metal {
        probe::detect_metal_device_available()
    } else {
        false
    };
    policy::resolve_device_kind_from_inputs(
        explicit_device.as_deref(),
        fallback_mode,
        metal_available,
    )
}

pub(in crate::llm::vision::deepseek) fn local_runtime_may_use_metal() -> bool {
    matches!(parse_device_kind(), DeviceKind::Metal)
}

pub(crate) fn resolve_device_kind_label_for_tests(
    explicit_device: Option<&str>,
    fallback_mode: AccelerationDevice,
    metal_available: bool,
) -> &'static str {
    match policy::resolve_device_kind_from_inputs(explicit_device, fallback_mode, metal_available) {
        DeviceKind::Cpu => "cpu",
        DeviceKind::Metal => "metal",
        DeviceKind::Cuda => "cuda",
    }
}
