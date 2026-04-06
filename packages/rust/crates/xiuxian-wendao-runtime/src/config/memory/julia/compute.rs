use crate::settings::{first_non_empty, get_setting_bool, get_setting_string, parse_positive_u64};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// Default Flight base URL for memory-family Julia compute services.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_BASE_URL: &str = "http://127.0.0.1:8815";
/// Default physical schema version for the memory-family Flight contract.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_SCHEMA_VERSION: &str = "v1";
/// Default plugin identifier for the memory-family Julia compute provider.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_PLUGIN_ID: &str = "wendao.memory";
/// Default timeout for memory-family Julia compute roundtrips.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_TIMEOUT_SECS: u64 = 10;
/// Default route for episodic recall compute requests.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_EPISODIC_RECALL_ROUTE: &str = "/memory/episodic_recall";
/// Default route for memory gate scoring requests.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_GATE_SCORE_ROUTE: &str = "/memory/gate_score";
/// Default route for memory plan tuning requests.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_PLAN_TUNING_ROUTE: &str = "/memory/plan_tuning";
/// Default route for memory calibration requests.
pub const DEFAULT_MEMORY_JULIA_COMPUTE_CALIBRATION_ROUTE: &str = "/memory/calibrate";

/// Fallback behavior for memory-family Julia compute integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryJuliaComputeFallbackMode {
    /// Fall back to the Rust-hosted implementation when Julia compute is unavailable.
    #[default]
    Rust,
}

impl MemoryJuliaComputeFallbackMode {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "rust" => Some(Self::Rust),
            _ => None,
        }
    }
}

/// Transport interaction mode requested from the Julia compute provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryJuliaComputeServiceMode {
    /// Expect stream-capable Flight handlers by default.
    #[default]
    Stream,
    /// Expect table-oriented Flight handlers.
    Table,
}

impl MemoryJuliaComputeServiceMode {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "stream" => Some(Self::Stream),
            "table" => Some(Self::Table),
            _ => None,
        }
    }
}

/// Route map for the first `memory` capability family profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryJuliaComputeRoutesRuntimeConfig {
    /// Route for read-only episodic recall compute.
    pub episodic_recall: String,
    /// Route for recommendation-only memory gate scoring.
    pub memory_gate_score: String,
    /// Route for advice-only memory plan tuning.
    pub memory_plan_tuning: String,
    /// Route for artifact-only memory calibration.
    pub memory_calibration: String,
}

impl Default for MemoryJuliaComputeRoutesRuntimeConfig {
    fn default() -> Self {
        Self {
            episodic_recall: DEFAULT_MEMORY_JULIA_COMPUTE_EPISODIC_RECALL_ROUTE.to_string(),
            memory_gate_score: DEFAULT_MEMORY_JULIA_COMPUTE_GATE_SCORE_ROUTE.to_string(),
            memory_plan_tuning: DEFAULT_MEMORY_JULIA_COMPUTE_PLAN_TUNING_ROUTE.to_string(),
            memory_calibration: DEFAULT_MEMORY_JULIA_COMPUTE_CALIBRATION_ROUTE.to_string(),
        }
    }
}

/// Runtime-owned memory-family Julia compute configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryJuliaComputeRuntimeConfig {
    /// Enable the external Julia compute lane.
    pub enabled: bool,
    /// Base URL for the memory-family Flight service.
    pub base_url: String,
    /// Physical schema version for request/response transport.
    pub schema_version: String,
    /// Provider identity advertised by the Julia capability manifest.
    pub plugin_id: String,
    /// Optional family-level health route for the Julia compute service.
    pub health_route: Option<String>,
    /// Transport interaction mode used by the host integration.
    pub service_mode: MemoryJuliaComputeServiceMode,
    /// Optional scenario pack forwarded into the Julia compute lane.
    pub scenario_pack: Option<String>,
    /// Timeout budget for one compute roundtrip.
    pub timeout_secs: u64,
    /// Fallback behavior when Julia compute is unavailable.
    pub fallback_mode: MemoryJuliaComputeFallbackMode,
    /// Whether the host should record shadow drift against the Rust baseline.
    pub shadow_compare: bool,
    /// Profile routes under the memory-family provider.
    pub routes: MemoryJuliaComputeRoutesRuntimeConfig,
}

impl Default for MemoryJuliaComputeRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: DEFAULT_MEMORY_JULIA_COMPUTE_BASE_URL.to_string(),
            schema_version: DEFAULT_MEMORY_JULIA_COMPUTE_SCHEMA_VERSION.to_string(),
            plugin_id: DEFAULT_MEMORY_JULIA_COMPUTE_PLUGIN_ID.to_string(),
            health_route: None,
            service_mode: MemoryJuliaComputeServiceMode::default(),
            scenario_pack: None,
            timeout_secs: DEFAULT_MEMORY_JULIA_COMPUTE_TIMEOUT_SECS,
            fallback_mode: MemoryJuliaComputeFallbackMode::default(),
            shadow_compare: true,
            routes: MemoryJuliaComputeRoutesRuntimeConfig::default(),
        }
    }
}

fn resolve_non_empty_string(settings: &Value, dotted_key: &str) -> Option<String> {
    first_non_empty(&[get_setting_string(settings, dotted_key)])
}

/// Resolve `memory.julia_compute` from merged Wendao settings.
#[must_use]
pub fn resolve_memory_julia_compute_runtime_with_settings(
    settings: &Value,
) -> MemoryJuliaComputeRuntimeConfig {
    let mut resolved = MemoryJuliaComputeRuntimeConfig::default();

    if let Some(enabled) = get_setting_bool(settings, "memory.julia_compute.enabled") {
        resolved.enabled = enabled;
    }

    if let Some(base_url) = resolve_non_empty_string(settings, "memory.julia_compute.base_url") {
        resolved.base_url = base_url;
    }

    if let Some(schema_version) =
        resolve_non_empty_string(settings, "memory.julia_compute.schema_version")
    {
        resolved.schema_version = schema_version;
    }

    if let Some(plugin_id) = resolve_non_empty_string(settings, "memory.julia_compute.plugin_id") {
        resolved.plugin_id = plugin_id;
    }

    resolved.health_route = resolve_non_empty_string(settings, "memory.julia_compute.health_route");

    if let Some(service_mode) =
        resolve_non_empty_string(settings, "memory.julia_compute.service_mode")
            .as_deref()
            .and_then(MemoryJuliaComputeServiceMode::parse)
    {
        resolved.service_mode = service_mode;
    }

    resolved.scenario_pack =
        resolve_non_empty_string(settings, "memory.julia_compute.scenario_pack");

    if let Some(timeout_secs) =
        resolve_non_empty_string(settings, "memory.julia_compute.timeout_secs")
            .as_deref()
            .and_then(parse_positive_u64)
    {
        resolved.timeout_secs = timeout_secs;
    }

    if let Some(fallback_mode) =
        resolve_non_empty_string(settings, "memory.julia_compute.fallback_mode")
            .as_deref()
            .and_then(MemoryJuliaComputeFallbackMode::parse)
    {
        resolved.fallback_mode = fallback_mode;
    }

    if let Some(shadow_compare) = get_setting_bool(settings, "memory.julia_compute.shadow_compare")
    {
        resolved.shadow_compare = shadow_compare;
    }

    if let Some(route) =
        resolve_non_empty_string(settings, "memory.julia_compute.routes.episodic_recall")
    {
        resolved.routes.episodic_recall = route;
    }

    if let Some(route) =
        resolve_non_empty_string(settings, "memory.julia_compute.routes.memory_gate_score")
    {
        resolved.routes.memory_gate_score = route;
    }

    if let Some(route) =
        resolve_non_empty_string(settings, "memory.julia_compute.routes.memory_plan_tuning")
    {
        resolved.routes.memory_plan_tuning = route;
    }

    if let Some(route) =
        resolve_non_empty_string(settings, "memory.julia_compute.routes.memory_calibration")
    {
        resolved.routes.memory_calibration = route;
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_MEMORY_JULIA_COMPUTE_BASE_URL, DEFAULT_MEMORY_JULIA_COMPUTE_EPISODIC_RECALL_ROUTE,
        DEFAULT_MEMORY_JULIA_COMPUTE_PLUGIN_ID, DEFAULT_MEMORY_JULIA_COMPUTE_SCHEMA_VERSION,
        DEFAULT_MEMORY_JULIA_COMPUTE_TIMEOUT_SECS, MemoryJuliaComputeFallbackMode,
        MemoryJuliaComputeServiceMode, resolve_memory_julia_compute_runtime_with_settings,
    };
    use crate::config::test_support;
    use std::fs;

    #[test]
    fn memory_julia_compute_runtime_reads_override_values() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[memory.julia_compute]
enabled = true
base_url = "grpc://127.0.0.1:18825"
schema_version = "v2"
plugin_id = "wendao.memory.shadow"
health_route = "/healthz"
service_mode = "table"
scenario_pack = "searchinfra"
timeout_secs = 3
fallback_mode = "rust"
shadow_compare = false

[memory.julia_compute.routes]
episodic_recall = "/memory/custom_recall"
memory_gate_score = "/memory/custom_gate_score"
memory_plan_tuning = "/memory/custom_plan_tuning"
memory_calibration = "/memory/custom_calibration"
"#,
        )?;

        let settings = test_support::load_test_settings_from_path(&config_path)?;
        let runtime = resolve_memory_julia_compute_runtime_with_settings(&settings);

        assert!(runtime.enabled);
        assert_eq!(runtime.base_url, "grpc://127.0.0.1:18825");
        assert_eq!(runtime.schema_version, "v2");
        assert_eq!(runtime.plugin_id, "wendao.memory.shadow");
        assert_eq!(runtime.health_route.as_deref(), Some("/healthz"));
        assert_eq!(runtime.service_mode, MemoryJuliaComputeServiceMode::Table);
        assert_eq!(runtime.scenario_pack.as_deref(), Some("searchinfra"));
        assert_eq!(runtime.timeout_secs, 3);
        assert_eq!(runtime.fallback_mode, MemoryJuliaComputeFallbackMode::Rust);
        assert!(!runtime.shadow_compare);
        assert_eq!(runtime.routes.episodic_recall, "/memory/custom_recall");
        assert_eq!(
            runtime.routes.memory_gate_score,
            "/memory/custom_gate_score"
        );
        assert_eq!(
            runtime.routes.memory_plan_tuning,
            "/memory/custom_plan_tuning"
        );
        assert_eq!(
            runtime.routes.memory_calibration,
            "/memory/custom_calibration"
        );

        Ok(())
    }

    #[test]
    fn memory_julia_compute_runtime_falls_back_on_blank_or_invalid_values()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[memory.julia_compute]
base_url = "   "
schema_version = "   "
plugin_id = ""
health_route = " "
service_mode = "invalid"
timeout_secs = 0
fallback_mode = "invalid"
shadow_compare = true

[memory.julia_compute.routes]
episodic_recall = "   "
"#,
        )?;

        let settings = test_support::load_test_settings_from_path(&config_path)?;
        let runtime = resolve_memory_julia_compute_runtime_with_settings(&settings);

        assert!(!runtime.enabled);
        assert_eq!(runtime.base_url, DEFAULT_MEMORY_JULIA_COMPUTE_BASE_URL);
        assert_eq!(
            runtime.schema_version,
            DEFAULT_MEMORY_JULIA_COMPUTE_SCHEMA_VERSION
        );
        assert_eq!(runtime.plugin_id, DEFAULT_MEMORY_JULIA_COMPUTE_PLUGIN_ID);
        assert_eq!(runtime.health_route, None);
        assert_eq!(runtime.service_mode, MemoryJuliaComputeServiceMode::Stream);
        assert_eq!(
            runtime.timeout_secs,
            DEFAULT_MEMORY_JULIA_COMPUTE_TIMEOUT_SECS
        );
        assert_eq!(runtime.fallback_mode, MemoryJuliaComputeFallbackMode::Rust);
        assert!(runtime.shadow_compare);
        assert_eq!(
            runtime.routes.episodic_recall,
            DEFAULT_MEMORY_JULIA_COMPUTE_EPISODIC_RECALL_ROUTE
        );

        Ok(())
    }
}
