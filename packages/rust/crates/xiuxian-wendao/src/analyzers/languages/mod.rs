//! Language-specific Repo Intelligence plugins bundled into the Wendao runtime.
//!
//! The Julia plugin now enters the host through a normal crate dependency.
//! The remaining path-inclusion seam is currently Modelica-specific and is
//! tracked separately under `M4`.

#[cfg(feature = "modelica")]
#[path = "../../../../xiuxian-wendao-modelica/src/plugin/mod.rs"]
mod modelica;

#[cfg(feature = "julia")]
pub use xiuxian_wendao_julia::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, JuliaRepoIntelligencePlugin,
    build_julia_arrow_transport_client, process_julia_arrow_batches,
    process_julia_arrow_batches_for_repository, register_into as register_julia_plugin,
    validate_julia_arrow_response_batches,
};

#[cfg(feature = "modelica")]
pub use modelica::{ModelicaRepoIntelligencePlugin, register_into as register_modelica_plugin};
