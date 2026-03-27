//! Language-specific Repo Intelligence plugins bundled into the Wendao runtime.
//!
//! The canonical Julia plugin implementation still lives in the sibling
//! workspace crate, but the gateway runtime needs that analyzer in the default
//! binary build. To avoid a cargo dependency cycle, we compile the source
//! modules into the core crate and register them through the builtin registry
//! surface.

#[cfg(feature = "julia")]
#[path = "../../../../xiuxian-wendao-julia/src/plugin/mod.rs"]
mod julia;

#[cfg(all(test, feature = "julia"))]
pub(crate) use julia::test_support as julia_plugin_test_support;

#[cfg(feature = "modelica")]
#[path = "../../../../xiuxian-wendao-modelica/src/plugin/mod.rs"]
mod modelica;

#[cfg(feature = "julia")]
pub use julia::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, JuliaRepoIntelligencePlugin,
    build_julia_arrow_transport_client, process_julia_arrow_batches,
    process_julia_arrow_batches_for_repository, register_into as register_julia_plugin,
    validate_julia_arrow_response_batches,
};

#[cfg(feature = "modelica")]
pub use modelica::{ModelicaRepoIntelligencePlugin, register_into as register_modelica_plugin};
