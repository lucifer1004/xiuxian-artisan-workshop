//! External Julia Repo Intelligence plugin for `xiuxian-wendao`.

/// Legacy compatibility surfaces exported by the standalone Julia plugin crate.
pub mod compatibility;
mod plugin;

#[cfg(test)]
pub(crate) use plugin::test_support as julia_plugin_test_support;

pub use plugin::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, JuliaRepoIntelligencePlugin,
    build_julia_flight_transport_client, process_julia_flight_batches,
    process_julia_flight_batches_for_repository, register_into,
    validate_julia_arrow_response_batches,
};
