use xiuxian_wendao_core::{
    PluginCapabilityBinding, PluginProviderSelector, artifacts::PluginArtifactSelector,
};
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH, DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH,
    LinkGraphJuliaRerankRuntimeConfig, build_rerank_provider_binding,
    julia_deployment_artifact_selector, julia_rerank_provider_selector,
};

/// Return the linked builtin Julia analyzer example config path.
#[must_use]
pub fn linked_builtin_julia_analyzer_example_config_path() -> &'static str {
    DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH
}

/// Return the linked builtin Julia analyzer launcher path.
#[must_use]
pub fn linked_builtin_julia_analyzer_launcher_path() -> &'static str {
    DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
}

/// Return the linked builtin Julia rerank provider selector.
#[must_use]
pub fn linked_builtin_julia_rerank_provider_selector() -> PluginProviderSelector {
    julia_rerank_provider_selector()
}

/// Return the linked builtin Julia deployment artifact selector.
#[must_use]
pub fn linked_builtin_julia_deployment_artifact_selector() -> PluginArtifactSelector {
    julia_deployment_artifact_selector()
}

/// Build a linked builtin Julia rerank binding from endpoint overrides.
#[must_use]
pub fn linked_builtin_julia_rerank_provider_binding_with_endpoint(
    base_url: &str,
    route: &str,
    health_route: &str,
    schema_version: &str,
    timeout_secs: u64,
) -> PluginCapabilityBinding {
    build_rerank_provider_binding(&LinkGraphJuliaRerankRuntimeConfig {
        base_url: Some(base_url.to_string()),
        route: Some(route.to_string()),
        health_route: Some(health_route.to_string()),
        schema_version: Some(schema_version.to_string()),
        timeout_secs: Some(timeout_secs),
        service_mode: None,
        analyzer_config_path: None,
        analyzer_strategy: None,
        vector_weight: None,
        similarity_weight: None,
    })
}
