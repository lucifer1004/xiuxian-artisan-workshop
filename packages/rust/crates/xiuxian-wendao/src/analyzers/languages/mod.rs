//! Language-specific Repo Intelligence plugins bundled into the Wendao runtime.
//!
//! The Julia and Modelica plugins now enter the host through normal crate
//! dependencies and self-registration. Their thick public APIs should be
//! consumed from the plugin crates directly rather than re-exported through
//! `xiuxian-wendao`.
