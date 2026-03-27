#[path = "runtime_config/constants.rs"]
mod constants;
#[path = "runtime_config/models.rs"]
pub(crate) mod models;
#[path = "runtime_config/resolve/mod.rs"]
pub mod resolve;
#[path = "runtime_config/settings/mod.rs"]
mod settings;

pub(crate) use constants::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;
pub(crate) use models::LinkGraphCacheRuntimeConfig;
pub use models::{
    LinkGraphIndexRuntimeConfig, LinkGraphJuliaAnalyzerLaunchManifest,
    LinkGraphJuliaAnalyzerServiceDescriptor, LinkGraphJuliaDeploymentArtifact,
    LinkGraphJuliaRerankRuntimeConfig,
};
pub use resolve::resolve_link_graph_index_runtime;
pub use resolve::{
    resolve_link_graph_agentic_runtime, resolve_link_graph_cache_runtime,
    resolve_link_graph_coactivation_runtime, resolve_link_graph_related_runtime,
};

pub(crate) use resolve::resolve_link_graph_retrieval_policy_runtime;
pub use settings::{set_link_graph_config_home_override, set_link_graph_wendao_config_override};

/// Resolve the current Julia rerank deployment artifact from Wendao runtime configuration.
#[must_use]
pub fn resolve_link_graph_julia_deployment_artifact() -> LinkGraphJuliaDeploymentArtifact {
    resolve_link_graph_retrieval_policy_runtime()
        .julia_rerank
        .deployment_artifact()
}

/// Resolve the current Julia rerank deployment artifact and render it as TOML.
///
/// # Errors
///
/// Returns an error when the resolved deployment artifact cannot be serialized
/// into TOML.
pub fn export_link_graph_julia_deployment_artifact_toml() -> Result<String, toml::ser::Error> {
    resolve_link_graph_julia_deployment_artifact().to_toml_string()
}

#[cfg(test)]
mod tests;
