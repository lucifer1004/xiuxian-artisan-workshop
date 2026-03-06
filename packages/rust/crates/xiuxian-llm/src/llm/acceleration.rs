//! Unified acceleration-device configuration for local model runtimes.
//!
//! This module provides one shared device selector (`auto`/`cpu`/`metal`/`cuda`)
//! so both DeepSeek OCR and mistralrs in-process runtime can consume the same
//! user intent from Xiuxian TOML or environment.

use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;
use tracing::warn;
use xiuxian_macros::env_non_empty;

/// Unified acceleration device mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelerationDevice {
    /// Let runtime pick the best available backend for the current platform.
    Auto,
    /// Force CPU.
    Cpu,
    /// Force Apple Metal.
    Metal,
    /// Force CUDA.
    Cuda,
}

impl AccelerationDevice {
    /// Stable string form used by logs and diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Metal => "metal",
            Self::Cuda => "cuda",
        }
    }
}

#[xiuxian_macros::xiuxian_config(
    namespace = "llm.acceleration",
    internal_path = "resources/config/acceleration.toml",
    orphan_file = ""
)]
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct AccelerationTomlConfig {
    device: Option<String>,
}

static CONFIG: OnceLock<AccelerationTomlConfig> = OnceLock::new();

fn config() -> &'static AccelerationTomlConfig {
    CONFIG.get_or_init(load_config)
}

fn load_config() -> AccelerationTomlConfig {
    AccelerationTomlConfig::load().unwrap_or_else(|error| {
        warn!(
            event = "llm.acceleration.config.load_failed",
            error = %error,
            "Acceleration config load failed, falling back to defaults"
        );
        AccelerationTomlConfig::default()
    })
}

/// Parse a raw acceleration mode token.
#[must_use]
pub fn parse_acceleration_device(raw: Option<&str>) -> Option<AccelerationDevice> {
    let normalized = raw.map(str::trim)?.to_ascii_lowercase();
    match normalized.as_str() {
        "auto" => Some(AccelerationDevice::Auto),
        "cpu" => Some(AccelerationDevice::Cpu),
        "metal" => Some(AccelerationDevice::Metal),
        "cuda" => Some(AccelerationDevice::Cuda),
        _ => None,
    }
}

/// Resolve unified acceleration device with precedence:
///
/// 1. explicit value provided by caller
/// 2. `XIUXIAN_ACCELERATION_DEVICE`
/// 3. `XIUXIAN_ACCEL_DEVICE`
/// 4. `llm.acceleration.device` from Xiuxian TOML
/// 5. fallback `auto`
#[must_use]
pub fn resolve_acceleration_device(explicit: Option<&str>) -> AccelerationDevice {
    resolve_acceleration_device_with(
        explicit,
        env_non_empty!("XIUXIAN_ACCELERATION_DEVICE").as_deref(),
        env_non_empty!("XIUXIAN_ACCEL_DEVICE").as_deref(),
        config().device.as_deref(),
    )
}

fn resolve_acceleration_device_with(
    explicit: Option<&str>,
    env_acceleration_device: Option<&str>,
    env_accel_device: Option<&str>,
    config_device: Option<&str>,
) -> AccelerationDevice {
    parse_acceleration_device(explicit)
        .or_else(|| parse_acceleration_device(env_acceleration_device))
        .or_else(|| parse_acceleration_device(env_accel_device))
        .or_else(|| parse_acceleration_device(config_device))
        .unwrap_or(AccelerationDevice::Auto)
}

pub(crate) fn resolve_acceleration_device_with_for_tests(
    explicit: Option<&str>,
    env_acceleration_device: Option<&str>,
    env_accel_device: Option<&str>,
    config_device: Option<&str>,
) -> AccelerationDevice {
    resolve_acceleration_device_with(
        explicit,
        env_acceleration_device,
        env_accel_device,
        config_device,
    )
}

pub(crate) fn load_config_with_paths_for_tests(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> Option<String> {
    AccelerationTomlConfig::load_with_paths(project_root, config_home)
        .ok()
        .and_then(|config| config.device)
}
