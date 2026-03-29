use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;
use xiuxian_vector::{ArrowTransportClient, ArrowTransportConfig};

/// Build an Arrow transport client from a generic plugin capability binding.
///
/// # Errors
///
/// Returns an error when the contract version or timeout cannot be translated
/// into a valid Arrow transport configuration, or when the transport client
/// cannot be constructed.
pub fn build_arrow_transport_client_from_binding(
    binding: &PluginCapabilityBinding,
) -> Result<Option<ArrowTransportClient>, String> {
    let Some(base_url) = binding.endpoint.base_url.as_deref() else {
        return Ok(None);
    };

    let mut resolved = ArrowTransportConfig::new(base_url);
    if let Some(route) = binding.endpoint.route.as_deref() {
        resolved = resolved.with_route(route);
    }
    if let Some(health_route) = binding.endpoint.health_route.as_deref() {
        resolved = resolved.with_health_route(health_route);
    }
    if !binding.contract_version.0.trim().is_empty() {
        resolved = resolved
            .with_schema_version(binding.contract_version.0.as_str())
            .map_err(|error| format!("invalid plugin transport schema version: {error}"))?;
    }
    if let Some(timeout_secs) = binding.endpoint.timeout_secs {
        resolved = resolved
            .with_timeout_secs(timeout_secs)
            .map_err(|error| format!("invalid plugin transport timeout: {error}"))?;
    }
    ArrowTransportClient::new(resolved)
        .map(Some)
        .map_err(|error| format!("failed to construct Arrow transport client: {error}"))
}

#[cfg(test)]
mod tests {
    use super::build_arrow_transport_client_from_binding;
    use xiuxian_wendao_core::{
        capabilities::{ContractVersion, PluginCapabilityBinding, PluginProviderSelector},
        ids::{CapabilityId, PluginId},
        transport::{PluginTransportEndpoint, PluginTransportKind},
    };

    fn sample_binding(base_url: Option<&str>) -> PluginCapabilityBinding {
        PluginCapabilityBinding {
            selector: PluginProviderSelector {
                capability_id: CapabilityId("rerank".to_string()),
                provider: PluginId("xiuxian-wendao-julia".to_string()),
            },
            endpoint: PluginTransportEndpoint {
                base_url: base_url.map(ToString::to_string),
                route: Some("/arrow-ipc".to_string()),
                health_route: Some("/healthz".to_string()),
                timeout_secs: Some(15),
            },
            launch: None,
            transport: PluginTransportKind::ArrowIpcHttp,
            contract_version: ContractVersion("v2".to_string()),
        }
    }

    #[test]
    fn transport_client_builder_returns_none_without_base_url() {
        let client = build_arrow_transport_client_from_binding(&sample_binding(None))
            .unwrap_or_else(|error| panic!("transport builder should not fail: {error}"));

        assert!(client.is_none());
    }

    #[test]
    fn transport_client_builder_applies_binding_overrides() {
        let client = build_arrow_transport_client_from_binding(&sample_binding(Some("http://127.0.0.1:18080")))
            .unwrap_or_else(|error| panic!("transport builder should succeed: {error}"))
            .unwrap_or_else(|| panic!("transport client should exist"));

        let config = client.config();
        assert_eq!(config.base_url(), "http://127.0.0.1:18080");
        assert_eq!(config.route(), "/arrow-ipc");
        assert_eq!(config.health_route(), "/healthz");
        assert_eq!(config.schema_version(), "v2");
        assert_eq!(config.timeout().as_secs(), 15);
    }
}
