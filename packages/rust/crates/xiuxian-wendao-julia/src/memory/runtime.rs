use xiuxian_wendao_core::{
    capabilities::{ContractVersion, PluginCapabilityBinding, PluginProviderSelector},
    ids::{CapabilityId, PluginId},
    repo_intelligence::RepoIntelligenceError,
    transport::{PluginTransportEndpoint, PluginTransportKind},
};
use xiuxian_wendao_runtime::{
    config::MemoryJuliaComputeRuntimeConfig,
    transport::{
        normalize_flight_route, validate_flight_schema_version, validate_flight_timeout_secs,
    },
};

use super::profile::MemoryJuliaComputeProfile;

/// Build one generic capability binding for a staged memory Julia compute profile.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the runtime config contains invalid
/// provider identity, route, schema version, or timeout values.
pub fn build_memory_julia_compute_binding(
    runtime: &MemoryJuliaComputeRuntimeConfig,
    profile: MemoryJuliaComputeProfile,
) -> Result<Option<PluginCapabilityBinding>, RepoIntelligenceError> {
    if !runtime.enabled {
        return Ok(None);
    }

    let provider = normalized_provider_id(runtime)?;
    let base_url = normalized_base_url(runtime)?;
    let health_route = normalized_health_route(runtime)?;
    let route = normalize_flight_route(route_for_profile(runtime, profile)).map_err(|error| {
        RepoIntelligenceError::ConfigLoad {
            message: format!(
                "memory Julia compute profile `{}` has invalid route: {error}",
                profile.profile_id()
            ),
        }
    })?;
    let schema_version =
        validate_flight_schema_version(&runtime.schema_version).map_err(|error| {
            RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "memory Julia compute profile `{}` has invalid schema version `{}`: {error}",
                    profile.profile_id(),
                    runtime.schema_version
                ),
            }
        })?;
    let timeout_secs = validate_flight_timeout_secs(runtime.timeout_secs).map_err(|error| {
        RepoIntelligenceError::ConfigLoad {
            message: format!(
                "memory Julia compute profile `{}` has invalid timeout `{}`: {error}",
                profile.profile_id(),
                runtime.timeout_secs
            ),
        }
    })?;

    Ok(Some(PluginCapabilityBinding {
        selector: PluginProviderSelector {
            capability_id: CapabilityId(profile.capability_id().to_string()),
            provider: PluginId(provider),
        },
        endpoint: PluginTransportEndpoint {
            base_url: Some(base_url),
            route: Some(route),
            health_route,
            timeout_secs: Some(timeout_secs),
        },
        launch: None,
        transport: PluginTransportKind::ArrowFlight,
        contract_version: ContractVersion(schema_version),
    }))
}

/// Build one binding per staged memory Julia compute profile.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the runtime config contains invalid
/// route, schema version, provider identity, or timeout values.
pub fn build_memory_julia_compute_bindings(
    runtime: &MemoryJuliaComputeRuntimeConfig,
) -> Result<Vec<PluginCapabilityBinding>, RepoIntelligenceError> {
    let mut bindings = Vec::new();
    for profile in MemoryJuliaComputeProfile::ALL {
        if let Some(binding) = build_memory_julia_compute_binding(runtime, profile)? {
            bindings.push(binding);
        }
    }
    Ok(bindings)
}

fn normalized_provider_id(
    runtime: &MemoryJuliaComputeRuntimeConfig,
) -> Result<String, RepoIntelligenceError> {
    let provider = runtime.plugin_id.trim();
    if provider.is_empty() {
        return Err(RepoIntelligenceError::ConfigLoad {
            message: "memory Julia compute plugin_id must not be blank".to_string(),
        });
    }
    Ok(provider.to_string())
}

fn normalized_base_url(
    runtime: &MemoryJuliaComputeRuntimeConfig,
) -> Result<String, RepoIntelligenceError> {
    let base_url = runtime.base_url.trim();
    if base_url.is_empty() {
        return Err(RepoIntelligenceError::ConfigLoad {
            message: "memory Julia compute base_url must not be blank".to_string(),
        });
    }
    Ok(base_url.to_string())
}

fn route_for_profile(
    runtime: &MemoryJuliaComputeRuntimeConfig,
    profile: MemoryJuliaComputeProfile,
) -> &str {
    match profile {
        MemoryJuliaComputeProfile::EpisodicRecall => runtime.routes.episodic_recall.as_str(),
        MemoryJuliaComputeProfile::MemoryGateScore => runtime.routes.memory_gate_score.as_str(),
        MemoryJuliaComputeProfile::MemoryPlanTuning => runtime.routes.memory_plan_tuning.as_str(),
        MemoryJuliaComputeProfile::MemoryCalibration => runtime.routes.memory_calibration.as_str(),
    }
}

fn normalized_health_route(
    runtime: &MemoryJuliaComputeRuntimeConfig,
) -> Result<Option<String>, RepoIntelligenceError> {
    let Some(health_route) = runtime.health_route.as_deref() else {
        return Ok(None);
    };
    let trimmed = health_route.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    normalize_flight_route(trimmed)
        .map(Some)
        .map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!("memory Julia compute health_route is invalid: {error}"),
        })
}

#[cfg(test)]
mod tests {
    use super::{build_memory_julia_compute_binding, build_memory_julia_compute_bindings};
    use crate::memory::MemoryJuliaComputeProfile;
    use xiuxian_wendao_runtime::config::MemoryJuliaComputeRuntimeConfig;

    #[test]
    fn build_memory_julia_compute_bindings_skips_disabled_runtime() {
        let runtime = MemoryJuliaComputeRuntimeConfig::default();
        let bindings = build_memory_julia_compute_bindings(&runtime)
            .unwrap_or_else(|error| panic!("bindings should resolve: {error}"));
        assert!(bindings.is_empty());
    }

    #[test]
    fn build_memory_julia_compute_bindings_materialize_all_profiles() {
        let mut runtime = MemoryJuliaComputeRuntimeConfig {
            enabled: true,
            ..MemoryJuliaComputeRuntimeConfig::default()
        };
        runtime.plugin_id = "wendao.memory".to_string();
        runtime.base_url = "http://127.0.0.1:18825".to_string();
        runtime.health_route = Some("/healthz".to_string());
        runtime.routes.episodic_recall = "/memory/episodic_recall".to_string();
        runtime.routes.memory_gate_score = "/memory/gate_score".to_string();
        runtime.routes.memory_plan_tuning = "/memory/plan_tuning".to_string();
        runtime.routes.memory_calibration = "/memory/calibrate".to_string();

        let bindings = build_memory_julia_compute_bindings(&runtime)
            .unwrap_or_else(|error| panic!("bindings should resolve: {error}"));
        assert_eq!(bindings.len(), 4);
        assert_eq!(bindings[0].selector.capability_id.0, "episodic_recall");
        assert_eq!(bindings[1].selector.capability_id.0, "memory_gate_score");
        assert_eq!(bindings[2].selector.capability_id.0, "memory_plan_tuning");
        assert_eq!(bindings[3].selector.capability_id.0, "memory_calibration");
        assert_eq!(bindings[0].selector.provider.0, "wendao.memory");
        assert_eq!(
            bindings[0].endpoint.route.as_deref(),
            Some("/memory/episodic_recall")
        );
        assert_eq!(
            bindings[0].endpoint.health_route.as_deref(),
            Some("/healthz")
        );
        assert_eq!(
            bindings[3].endpoint.route.as_deref(),
            Some("/memory/calibrate")
        );
    }

    #[test]
    fn build_memory_julia_compute_binding_rejects_invalid_runtime_values() {
        let mut runtime = MemoryJuliaComputeRuntimeConfig {
            enabled: true,
            ..MemoryJuliaComputeRuntimeConfig::default()
        };
        runtime.plugin_id = "  ".to_string();
        let Err(error) =
            build_memory_julia_compute_binding(&runtime, MemoryJuliaComputeProfile::EpisodicRecall)
        else {
            panic!("blank provider id should fail");
        };
        let message = error.to_string();
        assert!(message.contains("plugin_id"));

        runtime.plugin_id = "wendao.memory".to_string();
        runtime.health_route = Some("/".to_string());
        let Err(error) =
            build_memory_julia_compute_binding(&runtime, MemoryJuliaComputeProfile::EpisodicRecall)
        else {
            panic!("invalid health_route should fail");
        };
        assert!(error.to_string().contains("health_route"));

        runtime.health_route = None;
        runtime.routes.episodic_recall = "/".to_string();
        let Err(error) =
            build_memory_julia_compute_binding(&runtime, MemoryJuliaComputeProfile::EpisodicRecall)
        else {
            panic!("invalid route should fail");
        };
        assert!(error.to_string().contains("invalid route"));
    }
}
