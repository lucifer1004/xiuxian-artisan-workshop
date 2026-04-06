use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;
use xiuxian_wendao_runtime::{
    config::MemoryJuliaComputeRuntimeConfig,
    transport::{NegotiatedFlightTransportClient, negotiate_flight_transport_client_from_bindings},
};

use crate::memory::{MemoryJuliaComputeProfile, build_memory_julia_compute_binding};

/// Build one negotiated Arrow Flight client for a memory-family Julia compute
/// profile from runtime config.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the runtime config cannot be
/// converted into a valid transport binding or negotiated Flight client.
pub fn build_memory_julia_compute_flight_transport_client(
    runtime: &MemoryJuliaComputeRuntimeConfig,
    profile: MemoryJuliaComputeProfile,
) -> Result<Option<NegotiatedFlightTransportClient>, RepoIntelligenceError> {
    let Some(binding) = build_memory_julia_compute_binding(runtime, profile)? else {
        return Ok(None);
    };

    negotiate_flight_transport_client_from_bindings(&[binding]).map_err(|error| {
        RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to build memory Julia compute Flight client for profile `{}`: {error}",
                profile.profile_id()
            ),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::build_memory_julia_compute_flight_transport_client;
    use crate::memory::MemoryJuliaComputeProfile;
    use xiuxian_wendao_runtime::config::MemoryJuliaComputeRuntimeConfig;

    #[test]
    fn build_memory_julia_compute_flight_transport_client_returns_none_when_disabled() {
        let runtime = MemoryJuliaComputeRuntimeConfig::default();
        let client = build_memory_julia_compute_flight_transport_client(
            &runtime,
            MemoryJuliaComputeProfile::EpisodicRecall,
        )
        .unwrap_or_else(|error| panic!("disabled runtime should not error: {error}"));
        assert!(client.is_none());
    }

    #[test]
    fn build_memory_julia_compute_flight_transport_client_reads_profile_route() {
        let mut runtime = MemoryJuliaComputeRuntimeConfig {
            enabled: true,
            ..MemoryJuliaComputeRuntimeConfig::default()
        };
        runtime.base_url = "http://127.0.0.1:18825".to_string();
        runtime.plugin_id = "wendao.memory".to_string();
        runtime.routes.memory_gate_score = "/memory/gate_score".to_string();

        let client = build_memory_julia_compute_flight_transport_client(
            &runtime,
            MemoryJuliaComputeProfile::MemoryGateScore,
        )
        .unwrap_or_else(|error| panic!("runtime should negotiate: {error}"))
        .unwrap_or_else(|| panic!("enabled runtime should build a client"));

        assert_eq!(client.flight_base_url(), "http://127.0.0.1:18825");
        assert_eq!(client.flight_route(), "/memory/gate_score");
    }
}
