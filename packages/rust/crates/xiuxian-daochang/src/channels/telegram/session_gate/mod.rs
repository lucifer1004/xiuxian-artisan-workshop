//! Session-scoped foreground gating.

mod config;
mod core;
mod types;
mod valkey;

pub use types::SessionGate;
