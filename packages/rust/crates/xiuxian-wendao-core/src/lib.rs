//! Stable plugin contract records for the Wendao core/runtime split.

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
