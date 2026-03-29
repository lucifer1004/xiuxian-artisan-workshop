#[path = "runtime_config/constants.rs"]
mod constants;
#[path = "runtime_config/models/mod.rs"]
pub(crate) mod models;
#[path = "runtime_config/resolve/mod.rs"]
pub mod resolve;
#[path = "runtime_config/settings/mod.rs"]
mod settings;

#[cfg(test)]
use crate::link_graph::plugin_runtime::{
    render_plugin_artifact_toml_for_selector, resolve_plugin_artifact_for_selector,
};
pub(crate) use constants::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;
pub(crate) use models::LinkGraphCacheRuntimeConfig;
#[cfg(test)]
pub(crate) use models::retrieval::LinkGraphCompatDeploymentArtifact;
pub use models::{LinkGraphIndexRuntimeConfig, julia_deployment_artifact_selector};
pub use resolve::resolve_link_graph_index_runtime;
pub use resolve::{
    resolve_link_graph_agentic_runtime, resolve_link_graph_cache_runtime,
    resolve_link_graph_coactivation_runtime, resolve_link_graph_related_runtime,
};
use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;

pub(crate) use resolve::resolve_link_graph_retrieval_policy_runtime;
pub use settings::{set_link_graph_config_home_override, set_link_graph_wendao_config_override};

/// Resolve the current retrieval rerank binding through the generic plugin-runtime model.
#[must_use]
pub fn resolve_link_graph_rerank_binding() -> Option<PluginCapabilityBinding> {
    resolve_link_graph_retrieval_policy_runtime().rerank_binding()
}

/// Resolve the current compatibility deployment artifact from Wendao runtime configuration.
#[must_use]
#[cfg(test)]
pub fn resolve_link_graph_compat_deployment_artifact() -> LinkGraphCompatDeploymentArtifact {
    resolve_plugin_artifact_for_selector(&julia_deployment_artifact_selector())
        .expect("compatibility deployment artifact should resolve")
        .into()
}

/// Resolve the current compatibility deployment artifact and render it as TOML.
///
/// # Errors
///
/// Returns an error when the resolved deployment artifact cannot be serialized
/// into TOML.
#[cfg(test)]
pub fn export_link_graph_compat_deployment_artifact_toml() -> Result<String, toml::ser::Error> {
    Ok(
        render_plugin_artifact_toml_for_selector(&julia_deployment_artifact_selector())?
            .expect("compatibility deployment artifact should render"),
    )
}

#[cfg(test)]
mod tests;
