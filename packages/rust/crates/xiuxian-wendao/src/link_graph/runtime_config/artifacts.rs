#[cfg(feature = "julia")]
use crate::link_graph::runtime_config::settings::merged_wendao_settings;
#[cfg(feature = "julia")]
use xiuxian_wendao_builtin::{
    render_builtin_plugin_artifact_toml_for_selector_with_settings,
    resolve_builtin_plugin_artifact_for_selector_with_settings,
};
use xiuxian_wendao_core::artifacts::{PluginArtifactPayload, PluginArtifactSelector};

/// Resolve one plugin artifact through the current link-graph runtime configuration.
#[must_use]
pub fn resolve_link_graph_plugin_artifact_for_selector(
    selector: &PluginArtifactSelector,
) -> Option<PluginArtifactPayload> {
    #[cfg(feature = "julia")]
    {
        let settings = merged_wendao_settings();
        resolve_builtin_plugin_artifact_for_selector_with_settings(selector, &settings)
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
        let settings = merged_wendao_settings();
        render_builtin_plugin_artifact_toml_for_selector_with_settings(selector, &settings)
    }

    #[cfg(not(feature = "julia"))]
    {
        let _ = selector;
        Ok(None)
    }
}
