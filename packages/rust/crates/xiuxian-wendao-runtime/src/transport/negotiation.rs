use xiuxian_vector::EngineRecordBatch;
use xiuxian_wendao_core::{capabilities::PluginCapabilityBinding, transport::PluginTransportKind};

use super::client::build_arrow_flight_transport_client_from_binding;
use super::flight::ArrowFlightTransportClient;

/// Canonical runtime transport preference order for plugin capability bindings.
pub const CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER: [PluginTransportKind; 1] =
    [PluginTransportKind::ArrowFlight];

/// Negotiated runtime transport selection for one plugin client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedTransportSelection {
    /// Transport kind ultimately selected by the runtime.
    pub selected_transport: PluginTransportKind,
    /// Higher-preference transport kind that was skipped before selection.
    pub fallback_from: Option<PluginTransportKind>,
    /// Reason the runtime fell back from a higher-preference transport kind.
    pub fallback_reason: Option<String>,
}

/// Runtime-owned Flight transport client paired with its negotiated selection metadata.
#[derive(Clone)]
pub struct NegotiatedFlightTransportClient {
    client: ArrowFlightTransportClient,
    selection: NegotiatedTransportSelection,
}

impl NegotiatedFlightTransportClient {
    fn new(client: ArrowFlightTransportClient, selection: NegotiatedTransportSelection) -> Self {
        Self { client, selection }
    }

    /// Borrow the negotiated transport-selection metadata.
    #[must_use]
    pub fn selection(&self) -> &NegotiatedTransportSelection {
        &self.selection
    }

    /// Return the configured Arrow Flight base URL.
    #[must_use]
    pub fn flight_base_url(&self) -> &str {
        self.client.base_url()
    }

    /// Return the configured Arrow Flight route.
    #[must_use]
    pub fn flight_route(&self) -> &str {
        self.client.route()
    }

    /// Process one engine batch through the negotiated runtime transport.
    ///
    /// # Errors
    ///
    /// Returns an error when the negotiated client cannot send the request or
    /// decode the response.
    pub async fn process_batch(
        &self,
        batch: &EngineRecordBatch,
    ) -> Result<Vec<EngineRecordBatch>, String> {
        self.client.process_batch(batch).await
    }

    /// Process multiple engine batches through the negotiated runtime transport.
    ///
    /// # Errors
    ///
    /// Returns an error when the negotiated client cannot send the request or
    /// decode the response.
    pub async fn process_batches(
        &self,
        batches: &[EngineRecordBatch],
    ) -> Result<Vec<EngineRecordBatch>, String> {
        self.client.process_batches(batches).await
    }
}

/// Negotiate the preferred runtime transport client from a set of candidate bindings.
///
/// The runtime negotiates only `ArrowFlight`.
///
/// # Errors
///
/// Returns an error when the candidate set contains configured bindings but no
/// binding can be materialized into a supported runtime transport client.
pub fn negotiate_flight_transport_client_from_bindings(
    bindings: &[PluginCapabilityBinding],
) -> Result<Option<NegotiatedFlightTransportClient>, String> {
    if bindings.is_empty() {
        return Ok(None);
    }

    let mut ordered = bindings.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by_key(|(index, binding)| (transport_preference_rank(binding.transport), *index));

    let mut fallback_from = None;
    let mut fallback_reason = None;
    let mut saw_material_failure = false;

    for (_, binding) in ordered {
        match build_runtime_flight_transport_client_from_binding(binding) {
            Ok(Some(client)) => {
                return Ok(Some(NegotiatedFlightTransportClient::new(
                    client,
                    NegotiatedTransportSelection {
                        selected_transport: binding.transport,
                        fallback_from,
                        fallback_reason,
                    },
                )));
            }
            Ok(None) => {
                fallback_from.get_or_insert(binding.transport);
                fallback_reason.get_or_insert_with(|| {
                    format!(
                        "preferred transport {:?} is unavailable because the binding has no base_url",
                        binding.transport
                    )
                });
            }
            Err(error) => {
                fallback_from.get_or_insert(binding.transport);
                fallback_reason.get_or_insert_with(|| {
                    format!(
                        "preferred transport {:?} is unavailable: {error}",
                        binding.transport
                    )
                });
                saw_material_failure = true;
            }
        }
    }

    if saw_material_failure {
        return Err(fallback_reason.unwrap_or_else(|| {
            "no supported runtime transport client could be negotiated".to_string()
        }));
    }

    Ok(None)
}

fn build_runtime_flight_transport_client_from_binding(
    binding: &PluginCapabilityBinding,
) -> Result<Option<ArrowFlightTransportClient>, String> {
    build_arrow_flight_transport_client_from_binding(binding)
}

const fn transport_preference_rank(kind: PluginTransportKind) -> usize {
    match kind {
        PluginTransportKind::ArrowFlight => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER,
        negotiate_flight_transport_client_from_bindings,
    };
    use xiuxian_wendao_core::{
        capabilities::{ContractVersion, PluginCapabilityBinding, PluginProviderSelector},
        ids::{CapabilityId, PluginId},
        transport::{PluginTransportEndpoint, PluginTransportKind},
    };

    fn sample_binding(
        transport: PluginTransportKind,
        base_url: Option<&str>,
        route: &str,
    ) -> PluginCapabilityBinding {
        PluginCapabilityBinding {
            selector: PluginProviderSelector {
                capability_id: CapabilityId("rerank".to_string()),
                provider: PluginId("xiuxian-wendao-julia".to_string()),
            },
            endpoint: PluginTransportEndpoint {
                base_url: base_url.map(ToString::to_string),
                route: Some(route.to_string()),
                health_route: Some("/healthz".to_string()),
                timeout_secs: Some(15),
            },
            launch: None,
            transport,
            contract_version: ContractVersion("v2".to_string()),
        }
    }

    #[test]
    fn canonical_transport_preference_order_is_flight_first() {
        assert_eq!(
            CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER,
            [PluginTransportKind::ArrowFlight]
        );
    }

    #[test]
    fn negotiation_selects_arrow_flight_when_binding_is_materializable() {
        let negotiated = negotiate_flight_transport_client_from_bindings(&[sample_binding(
            PluginTransportKind::ArrowFlight,
            Some("http://127.0.0.1:18080"),
            "/flight",
        )])
        .unwrap_or_else(|error| panic!("transport negotiation should succeed: {error}"))
        .unwrap_or_else(|| panic!("transport negotiation should select the Flight client"));

        assert_eq!(
            negotiated.selection().selected_transport,
            PluginTransportKind::ArrowFlight
        );
        assert_eq!(negotiated.selection().fallback_from, None);
        assert_eq!(negotiated.flight_base_url(), "http://127.0.0.1:18080");
        assert_eq!(negotiated.flight_route(), "/flight");
    }

    #[test]
    fn negotiation_reports_flight_materialization_errors_without_ipc_fallback() {
        let result = negotiate_flight_transport_client_from_bindings(&[sample_binding(
            PluginTransportKind::ArrowFlight,
            Some("http://127.0.0.1:18080"),
            "",
        )]);

        let error = match result {
            Ok(value) => panic!(
                "incomplete Flight bindings should now fail directly: {value_present}",
                value_present = value.is_some()
            ),
            Err(error) => error,
        };

        assert!(
            error.contains("preferred transport ArrowFlight is unavailable"),
            "expected transport negotiation failure context, got: {error}"
        );
        assert!(
            error.contains("failed to construct Arrow Flight client"),
            "expected Arrow Flight client construction error, got: {error}"
        );
        assert!(
            error.contains("at least one descriptor segment"),
            "expected Flight materialization error, got: {error}"
        );
    }

    #[test]
    fn negotiation_returns_none_when_candidate_bindings_are_unconfigured() {
        let negotiated = negotiate_flight_transport_client_from_bindings(&[sample_binding(
            PluginTransportKind::ArrowFlight,
            None,
            "/flight",
        )])
        .unwrap_or_else(|error| panic!("transport negotiation should not fail: {error}"));

        assert!(negotiated.is_none());
    }
}
