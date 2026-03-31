#[cfg(feature = "julia")]
mod client;
mod endpoint;
mod kind;
#[cfg(feature = "julia")]
mod server;

#[cfg(feature = "julia")]
pub use client::{
    CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, NegotiatedArrowTransportClient,
    NegotiatedTransportSelection, build_arrow_transport_client_from_binding,
    negotiate_arrow_transport_client_from_bindings,
};
pub use endpoint::PluginTransportEndpoint;
pub use kind::PluginTransportKind;
#[cfg(feature = "julia")]
pub use server::{
    SearchPlaneRepoSearchFlightRouteProvider, bootstrap_sample_repo_search_content,
    build_search_plane_flight_service,
};
