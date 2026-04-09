//! Stable shared contracts for the Wendao package split.
//!
//! Ownership rule:
//! - put stable identifiers, descriptors, traits, and payload records here
//! - do not put runtime config resolution, transport negotiation, host
//!   lifecycle, or Wendao business logic here
//!
//! `xiuxian-wendao-core` is intended to be consumable by runtime helpers and
//! plugin crates without pulling in deployment-dependent behavior.

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

/// Stable artifact payload and launch-spec records.
pub mod artifacts;
/// Stable capability-binding and contract-version records.
pub mod capabilities;
/// Stable plugin, capability, and artifact identifiers.
pub mod ids;
/// Stable repo-intelligence contracts shared by Wendao plugins.
pub mod repo_intelligence;
/// Stable transport endpoint and transport kind records.
pub mod transport;

pub use artifacts::{PluginArtifactPayload, PluginArtifactSelector, PluginLaunchSpec};
pub use capabilities::{ContractVersion, PluginCapabilityBinding, PluginProviderSelector};
pub use ids::{ArtifactId, CapabilityId, PluginId};
pub use transport::{PluginTransportEndpoint, PluginTransportKind};
