use xiuxian_vector::{ArrowTransportClient, ArrowTransportConfig, EngineRecordBatch};
use xiuxian_wendao_core::{
    capabilities::PluginCapabilityBinding,
    transport::PluginTransportKind,
};

use super::client::{
    build_arrow_flight_transport_client_from_binding, build_arrow_transport_client_from_binding,
};
use super::flight::ArrowFlightTransportClient;

/// Canonical runtime transport preference order for plugin capability bindings.
pub const CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER: [PluginTransportKind; 3] = [
    PluginTransportKind::ArrowFlight,
    PluginTransportKind::ArrowIpcHttp,
    PluginTransportKind::LocalProcessArrowIpc,
];

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

/// Runtime-owned Arrow transport client paired with its negotiated selection metadata.
#[derive(Clone)]
pub struct NegotiatedArrowTransportClient {
    client: RuntimeArrowTransportClient,
    selection: NegotiatedTransportSelection,
}

impl NegotiatedArrowTransportClient {
    fn new(client: RuntimeArrowTransportClient, selection: NegotiatedTransportSelection) -> Self {
        Self { client, selection }
    }

    /// Borrow the negotiated transport-selection metadata.
    #[must_use]
    pub fn selection(&self) -> &NegotiatedTransportSelection {
        &self.selection
    }

    /// Return the Arrow IPC-over-HTTP config when that transport was selected.
    #[must_use]
    pub fn arrow_ipc_http_config(&self) -> Option<&ArrowTransportConfig> {
        match &self.client {
            RuntimeArrowTransportClient::ArrowIpcHttp(client) => Some(client.config()),
            RuntimeArrowTransportClient::ArrowFlight(_) => None,
        }
    }

    /// Return the configured Arrow Flight base URL when that transport was selected.
    #[must_use]
    pub fn flight_base_url(&self) -> Option<&str> {
        match &self.client {
            RuntimeArrowTransportClient::ArrowIpcHttp(_) => None,
            RuntimeArrowTransportClient::ArrowFlight(client) => Some(client.base_url()),
        }
    }

    /// Return the configured Arrow Flight route when that transport was selected.
    #[must_use]
    pub fn flight_route(&self) -> Option<&str> {
        match &self.client {
            RuntimeArrowTransportClient::ArrowIpcHttp(_) => None,
            RuntimeArrowTransportClient::ArrowFlight(client) => Some(client.route()),
        }
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
        match &self.client {
            RuntimeArrowTransportClient::ArrowIpcHttp(client) => client
                .process_batch(batch)
                .await
                .map_err(|error| format!("Arrow IPC transport failed: {error}")),
            RuntimeArrowTransportClient::ArrowFlight(client) => client.process_batch(batch).await,
        }
    }
}

#[derive(Clone)]
enum RuntimeArrowTransportClient {
    ArrowIpcHttp(ArrowTransportClient),
    ArrowFlight(ArrowFlightTransportClient),
}

/// Negotiate the preferred runtime transport client from a set of candidate bindings.
///
/// The runtime currently prefers `ArrowFlight`, then `ArrowIpcHttp`, then
/// `LocalProcessArrowIpc`. Only `ArrowIpcHttp` is materializable into a live
/// client today, so higher-preference kinds deterministically fall back when a
/// lower-preference compatible binding is present.
///
/// # Errors
///
/// Returns an error when the candidate set contains configured bindings but no
/// binding can be materialized into a supported runtime transport client.
pub fn negotiate_arrow_transport_client_from_bindings(
    bindings: &[PluginCapabilityBinding],
) -> Result<Option<NegotiatedArrowTransportClient>, String> {
    if bindings.is_empty() {
        return Ok(None);
    }

    let mut ordered = bindings.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by_key(|(index, binding)| (transport_preference_rank(binding.transport), *index));

    let mut fallback_from = None;
    let mut fallback_reason = None;
    let mut saw_material_failure = false;

    for (_, binding) in ordered {
        match build_runtime_arrow_transport_client_from_binding(binding) {
            Ok(Some(client)) => {
                return Ok(Some(NegotiatedArrowTransportClient::new(
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
        return Err(
            fallback_reason
                .unwrap_or_else(|| "no supported runtime transport client could be negotiated".to_string()),
        );
    }

    Ok(None)
}

fn build_runtime_arrow_transport_client_from_binding(
    binding: &PluginCapabilityBinding,
) -> Result<Option<RuntimeArrowTransportClient>, String> {
    match binding.transport {
        PluginTransportKind::ArrowFlight => build_arrow_flight_transport_client_from_binding(binding)
            .map(|client| client.map(RuntimeArrowTransportClient::ArrowFlight)),
        PluginTransportKind::ArrowIpcHttp => build_arrow_transport_client_from_binding(binding)
            .map(|client| client.map(RuntimeArrowTransportClient::ArrowIpcHttp)),
        PluginTransportKind::LocalProcessArrowIpc => Err(
            "LocalProcessArrowIpc is not yet materializable on the runtime negotiation seam"
                .to_string(),
        ),
    }
}

const fn transport_preference_rank(kind: PluginTransportKind) -> usize {
    match kind {
        PluginTransportKind::ArrowFlight => 0,
        PluginTransportKind::ArrowIpcHttp => 1,
        PluginTransportKind::LocalProcessArrowIpc => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, negotiate_arrow_transport_client_from_bindings,
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
            [
                PluginTransportKind::ArrowFlight,
                PluginTransportKind::ArrowIpcHttp,
                PluginTransportKind::LocalProcessArrowIpc,
            ]
        );
    }

    #[test]
    fn negotiation_selects_arrow_flight_when_binding_is_materializable() {
        let negotiated = negotiate_arrow_transport_client_from_bindings(&[sample_binding(
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
        assert_eq!(negotiated.flight_base_url(), Some("http://127.0.0.1:18080"));
        assert_eq!(negotiated.flight_route(), Some("/flight"));
        assert!(negotiated.arrow_ipc_http_config().is_none());
    }

    #[test]
    fn negotiation_falls_back_to_arrow_ipc_when_flight_binding_is_incomplete() {
        let negotiated = negotiate_arrow_transport_client_from_bindings(&[
            sample_binding(
                PluginTransportKind::ArrowFlight,
                Some("http://127.0.0.1:18080"),
                "",
            ),
            sample_binding(
                PluginTransportKind::ArrowIpcHttp,
                Some("http://127.0.0.1:18081"),
                "/arrow-ipc",
            ),
        ])
        .unwrap_or_else(|error| panic!("transport negotiation should succeed: {error}"))
        .unwrap_or_else(|| panic!("transport negotiation should select a fallback client"));

        let selection = negotiated.selection();
        let config = negotiated
            .arrow_ipc_http_config()
            .unwrap_or_else(|| panic!("fallback should produce an Arrow IPC client"));
        assert_eq!(selection.selected_transport, PluginTransportKind::ArrowIpcHttp);
        assert_eq!(selection.fallback_from, Some(PluginTransportKind::ArrowFlight));
        assert!(
            selection
                .fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("ArrowFlight"),
            "expected fallback reason to mention ArrowFlight: {selection:?}"
        );
        assert_eq!(config.base_url(), "http://127.0.0.1:18081");
        assert_eq!(config.route(), "/arrow-ipc");
    }

    #[test]
    fn negotiation_returns_none_when_candidate_bindings_are_unconfigured() {
        let negotiated = negotiate_arrow_transport_client_from_bindings(&[sample_binding(
            PluginTransportKind::ArrowFlight,
            None,
            "/flight",
        )])
        .unwrap_or_else(|error| panic!("transport negotiation should not fail: {error}"));

        assert!(negotiated.is_none());
    }
}
