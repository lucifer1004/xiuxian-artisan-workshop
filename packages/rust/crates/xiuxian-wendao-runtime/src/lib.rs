//! Runtime-owned host behavior for the Wendao core/runtime split.

/// Runtime-owned artifact render helpers.
pub mod artifacts;
/// Runtime-owned live link-graph config records and resolvers.
pub mod runtime_config;
/// Runtime-owned config settings merge, override, and parsing helpers.
pub mod settings;
/// Transport negotiation and client-construction helpers.
pub mod transport;

#[cfg(test)]
mod tests;
