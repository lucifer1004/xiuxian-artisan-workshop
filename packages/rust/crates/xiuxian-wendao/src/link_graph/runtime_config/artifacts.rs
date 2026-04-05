#[cfg(feature = "julia")]
use crate::link_graph::runtime_config::resolve_link_graph_retrieval_policy_runtime;
use xiuxian_wendao_core::artifacts::{PluginArtifactPayload, PluginArtifactSelector};
#[cfg(feature = "julia")]
use xiuxian_wendao_julia::compatibility::link_graph::{
    julia_deployment_artifact_selector, render_julia_plugin_artifact_toml_for_selector,
    resolve_julia_plugin_artifact_payload_for_selector,
};

/// Resolve one plugin artifact through the current link-graph runtime configuration.
#[must_use]
pub fn resolve_link_graph_plugin_artifact_for_selector(
    selector: &PluginArtifactSelector,
) -> Option<PluginArtifactPayload> {
    #[cfg(feature = "julia")]
    {
        if selector != &julia_deployment_artifact_selector() {
            return None;
        }

        let runtime = resolve_link_graph_retrieval_policy_runtime();
        resolve_julia_plugin_artifact_payload_for_selector(selector, &runtime.julia_rerank)
    }

    #[cfg(not(feature = "julia"))]
    {
        let _ = selector;
        None
    }
}

/// Render a resolved link-graph plugin artifact as pretty TOML.
///
/// # Errors
///
/// Returns an error when the resolved artifact cannot be serialized into TOML.
pub fn render_link_graph_plugin_artifact_toml_for_selector(
    selector: &PluginArtifactSelector,
) -> Result<Option<String>, toml::ser::Error> {
    #[cfg(feature = "julia")]
    {
        if selector != &julia_deployment_artifact_selector() {
            return Ok(None);
        }

        let runtime = resolve_link_graph_retrieval_policy_runtime();
        render_julia_plugin_artifact_toml_for_selector(selector, &runtime.julia_rerank)
    }

    #[cfg(not(feature = "julia"))]
    {
        let _ = selector;
        Ok(None)
    }
}
