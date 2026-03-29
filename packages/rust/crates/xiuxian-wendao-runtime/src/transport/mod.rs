#[cfg(feature = "julia")]
mod client;

#[cfg(feature = "julia")]
pub use client::build_arrow_transport_client_from_binding;
