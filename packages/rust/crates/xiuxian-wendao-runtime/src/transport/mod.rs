#[cfg(feature = "julia")]
mod client;
#[cfg(feature = "julia")]
mod flight;
#[cfg(feature = "julia")]
mod negotiation;

#[cfg(feature = "julia")]
pub use client::build_arrow_transport_client_from_binding;
#[cfg(feature = "julia")]
pub use negotiation::{
    CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, NegotiatedArrowTransportClient,
    NegotiatedTransportSelection, negotiate_arrow_transport_client_from_bindings,
};
