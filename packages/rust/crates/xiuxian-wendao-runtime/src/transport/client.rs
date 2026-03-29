use std::time::Duration;

use xiuxian_vector::{
    ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION, ArrowTransportClient, ArrowTransportConfig,
};
use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;
use xiuxian_wendao_core::transport::PluginTransportKind;

use super::flight::ArrowFlightTransportClient;

/// Build an Arrow transport client from a generic plugin capability binding.
///
/// # Errors
///
/// Returns an error when the contract version or timeout cannot be translated
/// into a valid Arrow transport configuration, when the binding requests an
/// unsupported runtime transport kind, or when the transport client cannot be
/// constructed.
pub fn build_arrow_transport_client_from_binding(
    binding: &PluginCapabilityBinding,
) -> Result<Option<ArrowTransportClient>, String> {
    let Some(base_url) = binding.endpoint.base_url.as_deref() else {
        return Ok(None);
    };

    if binding.transport != PluginTransportKind::ArrowIpcHttp {
        return Err(format!(
            "unsupported plugin transport for Arrow IPC client construction: {:?}",
            binding.transport
        ));
    }

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

/// Build an Arrow Flight transport client from a generic plugin capability binding.
///
/// # Errors
///
/// Returns an error when the contract version or timeout cannot be translated
/// into a valid Arrow transport configuration, when the binding requests an
/// unsupported runtime transport kind, or when the Flight client cannot be
/// constructed.
pub(crate) fn build_arrow_flight_transport_client_from_binding(
    binding: &PluginCapabilityBinding,
) -> Result<Option<ArrowFlightTransportClient>, String> {
    let Some(base_url) = binding.endpoint.base_url.as_deref() else {
        return Ok(None);
    };

    if binding.transport != PluginTransportKind::ArrowFlight {
        return Err(format!(
            "unsupported plugin transport for Arrow Flight client construction: {:?}",
            binding.transport
        ));
    }

    let Some(route) = binding.endpoint.route.as_deref() else {
        return Err(
            "Arrow Flight client construction requires a route-backed FlightDescriptor path"
                .to_string(),
        );
    };

    let schema_version = normalized_schema_version(binding)?;
    let timeout = normalized_timeout(base_url, binding)?;

    ArrowFlightTransportClient::new(base_url, route, schema_version, timeout)
        .map(Some)
        .map_err(|error| format!("failed to construct Arrow Flight client: {error}"))
}

fn normalized_schema_version(binding: &PluginCapabilityBinding) -> Result<String, String> {
    if binding.contract_version.0.trim().is_empty() {
        return Ok(ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION.to_string());
    }

    ArrowTransportConfig::new(
        binding
            .endpoint
            .base_url
            .as_deref()
            .unwrap_or_default()
            .to_string(),
    )
    .with_schema_version(binding.contract_version.0.as_str())
    .map(|config| config.schema_version().to_string())
    .map_err(|error| format!("invalid plugin transport schema version: {error}"))
}

fn normalized_timeout(
    base_url: &str,
    binding: &PluginCapabilityBinding,
) -> Result<Duration, String> {
    let mut defaults = ArrowTransportConfig::new(base_url.to_string());
    if let Some(timeout_secs) = binding.endpoint.timeout_secs {
        defaults = defaults
            .with_timeout_secs(timeout_secs)
            .map_err(|error| format!("invalid plugin transport timeout: {error}"))?;
    }
    Ok(defaults.timeout())
}

#[cfg(test)]
mod tests {
    use super::{
        build_arrow_flight_transport_client_from_binding, build_arrow_transport_client_from_binding,
    };
    use xiuxian_vector::ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION;
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
        let client = build_arrow_transport_client_from_binding(&sample_binding(Some(
            "http://127.0.0.1:18080",
        )))
        .unwrap_or_else(|error| panic!("transport builder should succeed: {error}"))
        .unwrap_or_else(|| panic!("transport client should exist"));

        let config = client.config();
        assert_eq!(config.base_url(), "http://127.0.0.1:18080");
        assert_eq!(config.route(), "/arrow-ipc");
        assert_eq!(config.health_route(), "/healthz");
        assert_eq!(config.schema_version(), "v2");
        assert_eq!(config.timeout().as_secs(), 15);
    }

    #[test]
    fn transport_client_builder_rejects_unsupported_transport_kinds() {
        let result = build_arrow_transport_client_from_binding(&PluginCapabilityBinding {
            transport: PluginTransportKind::ArrowFlight,
            ..sample_binding(Some("http://127.0.0.1:18080"))
        });
        let error = match result {
            Ok(_) => panic!("Arrow Flight should not build through the Arrow IPC client seam"),
            Err(error) => error,
        };

        assert!(error.contains("unsupported plugin transport"));
        assert!(error.contains("ArrowFlight"));
    }

    #[test]
    fn flight_transport_client_builder_requires_a_route_descriptor() {
        let result = build_arrow_flight_transport_client_from_binding(&PluginCapabilityBinding {
            transport: PluginTransportKind::ArrowFlight,
            endpoint: PluginTransportEndpoint {
                route: None,
                ..sample_binding(Some("http://127.0.0.1:18080")).endpoint
            },
            ..sample_binding(Some("http://127.0.0.1:18080"))
        });
        let error = match result {
            Ok(_) => panic!("Arrow Flight construction should require an explicit route"),
            Err(error) => error,
        };

        assert!(error.contains("FlightDescriptor"));
    }

    #[test]
    fn flight_transport_client_builder_uses_arrow_defaults_for_schema_and_timeout() {
        let mut binding = sample_binding(Some("http://127.0.0.1:18080"));
        binding.endpoint.timeout_secs = None;
        let client = build_arrow_flight_transport_client_from_binding(&PluginCapabilityBinding {
            transport: PluginTransportKind::ArrowFlight,
            contract_version: ContractVersion(String::new()),
            ..binding
        })
        .unwrap_or_else(|error| panic!("flight transport builder should succeed: {error}"))
        .unwrap_or_else(|| panic!("flight transport client should exist"));

        assert_eq!(client.base_url(), "http://127.0.0.1:18080");
        assert_eq!(client.route(), "/arrow-ipc");
        assert_eq!(client.schema_version(), ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION);
        assert_eq!(client.timeout().as_secs(), 10);
    }
}
