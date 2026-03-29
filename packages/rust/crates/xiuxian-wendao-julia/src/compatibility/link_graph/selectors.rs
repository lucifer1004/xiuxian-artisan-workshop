use xiuxian_wendao_core::{
    artifacts::PluginArtifactSelector,
    capabilities::PluginProviderSelector,
    ids::{ArtifactId, CapabilityId, PluginId},
};

/// Stable plugin id used by the Julia compatibility path.
pub const JULIA_PLUGIN_ID: &str = "xiuxian-wendao-julia";
/// Stable capability id used by the Julia rerank compatibility path.
pub const JULIA_RERANK_CAPABILITY_ID: &str = "rerank";
/// Stable artifact id used by the Julia deployment compatibility path.
pub const JULIA_DEPLOYMENT_ARTIFACT_ID: &str = "deployment";

/// Build the canonical rerank capability selector for the Julia plugin.
#[must_use]
pub fn julia_rerank_provider_selector() -> PluginProviderSelector {
    PluginProviderSelector {
        capability_id: CapabilityId(JULIA_RERANK_CAPABILITY_ID.to_string()),
        provider: PluginId(JULIA_PLUGIN_ID.to_string()),
    }
}

/// Build the canonical deployment-artifact selector for the Julia plugin.
#[must_use]
pub fn julia_deployment_artifact_selector() -> PluginArtifactSelector {
    PluginArtifactSelector {
        plugin_id: PluginId(JULIA_PLUGIN_ID.to_string()),
        artifact_id: ArtifactId(JULIA_DEPLOYMENT_ARTIFACT_ID.to_string()),
    }
}
