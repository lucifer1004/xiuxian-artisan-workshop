use crate::link_graph::runtime_config::{
    julia_deployment_artifact_selector, resolve_link_graph_retrieval_policy_runtime,
};
use xiuxian_wendao_core::artifacts::{PluginArtifactPayload, PluginArtifactSelector};
use xiuxian_wendao_runtime::artifacts::{
    resolve_plugin_artifact_for_selector_with, resolve_plugin_artifact_with,
};

/// Resolve one plugin artifact through the current runtime compatibility layer.
#[must_use]
pub fn resolve_plugin_artifact(
    plugin_id: &str,
    artifact_id: &str,
) -> Option<PluginArtifactPayload> {
    resolve_plugin_artifact_with(plugin_id, artifact_id, resolve_plugin_artifact_for_selector)
}

/// Resolve one plugin artifact through the current runtime compatibility layer.
#[must_use]
pub fn resolve_plugin_artifact_for_selector(
    selector: &PluginArtifactSelector,
) -> Option<PluginArtifactPayload> {
    resolve_plugin_artifact_for_selector_with(selector, |selector| {
        if selector == &julia_deployment_artifact_selector() {
            Some(
                resolve_link_graph_retrieval_policy_runtime()
                    .julia_rerank
                    .plugin_artifact_payload(),
            )
        } else {
            None
        }
    })
}
