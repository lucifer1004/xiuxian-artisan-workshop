//! Language-specific Repo Intelligence plugins bundled into the Wendao runtime.
//!
//! The Julia and Modelica plugins now enter the host through normal crate
//! dependencies and self-registration. Their thick public APIs should be
//! consumed from the plugin crates directly rather than re-exported through
//! `xiuxian-wendao`.
// Keep builtin plugin crates linked so their `inventory`-submitted registrars
// remain visible to the host bootstrap without widening the public API again.
#[cfg(feature = "julia")]
use xiuxian_wendao_julia as _;
#[cfg(feature = "modelica")]
use xiuxian_wendao_modelica as _;
