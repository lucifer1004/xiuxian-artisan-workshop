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

#[cfg(feature = "modelica")]
#[path = "../../../../xiuxian-wendao-modelica/src/plugin/mod.rs"]
mod modelica;

#[cfg(feature = "julia")]
pub use julia::{JuliaRepoIntelligencePlugin, register_into as register_julia_plugin};

#[cfg(feature = "modelica")]
pub use modelica::{ModelicaRepoIntelligencePlugin, register_into as register_modelica_plugin};
