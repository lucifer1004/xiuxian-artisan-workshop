#[path = "runtime_config/artifacts.rs"]
mod artifacts;
#[path = "runtime_config/constants.rs"]
mod constants;
#[path = "runtime_config/models/mod.rs"]
pub(crate) mod models;
#[path = "runtime_config/resolve/mod.rs"]
pub mod resolve;
#[path = "runtime_config/settings/mod.rs"]
mod settings;

pub use artifacts::{
    render_link_graph_plugin_artifact_toml_for_selector,
    resolve_link_graph_plugin_artifact_for_selector,
};
pub(crate) use constants::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;
pub(crate) use models::LinkGraphCacheRuntimeConfig;
pub use models::LinkGraphIndexRuntimeConfig;
pub use resolve::resolve_link_graph_index_runtime;
pub use resolve::{
    resolve_link_graph_agentic_runtime, resolve_link_graph_cache_runtime,
    resolve_link_graph_coactivation_runtime, resolve_link_graph_related_runtime,
};
use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;
use xiuxian_wendao_runtime::transport::RerankScoreWeights;

pub(crate) use resolve::resolve_link_graph_retrieval_policy_runtime;
pub use settings::{set_link_graph_config_home_override, set_link_graph_wendao_config_override};

/// File-backed runtime settings that can influence the Flight rerank host.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkGraphRerankFlightRuntimeSettings {
    /// Schema version from retrieval policy config, if configured.
    pub schema_version: Option<String>,
    /// Score weights from retrieval policy config, if configured.
    pub score_weights: Option<RerankScoreWeights>,
}

/// Resolve the current retrieval rerank binding through the generic plugin-runtime model.
#[must_use]
pub fn resolve_link_graph_rerank_binding() -> Option<PluginCapabilityBinding> {
    resolve_link_graph_retrieval_policy_runtime().rerank_binding()
}

/// Resolve the current runtime-owned rerank score weights from Wendao
/// retrieval policy settings.
#[must_use]
pub fn resolve_link_graph_rerank_score_weights() -> Option<RerankScoreWeights> {
    #[cfg(feature = "julia")]
    {
        let runtime = resolve_link_graph_retrieval_policy_runtime();
        let defaults = RerankScoreWeights::default();
        let vector_weight = runtime.julia_rerank.vector_weight;
        let similarity_weight = runtime.julia_rerank.similarity_weight;

        if vector_weight.is_none() && similarity_weight.is_none() {
            return None;
        }

        RerankScoreWeights::new(
            vector_weight.unwrap_or(defaults.vector_weight),
            similarity_weight.unwrap_or(defaults.semantic_weight),
        )
        .ok()
    }

    #[cfg(not(feature = "julia"))]
    {
        None
    }
}

/// Resolve the current rerank-side schema version from Wendao retrieval
/// policy settings.
#[must_use]
pub fn resolve_link_graph_rerank_schema_version() -> Option<String> {
    #[cfg(feature = "julia")]
    {
        resolve_link_graph_retrieval_policy_runtime()
            .julia_rerank
            .schema_version
            .filter(|value| !value.trim().is_empty())
    }

    #[cfg(not(feature = "julia"))]
    {
        None
    }
}

/// Resolve the current file-backed Flight rerank host settings from Wendao
/// retrieval policy configuration.
#[must_use]
pub fn resolve_link_graph_rerank_flight_runtime_settings() -> LinkGraphRerankFlightRuntimeSettings {
    LinkGraphRerankFlightRuntimeSettings {
        schema_version: resolve_link_graph_rerank_schema_version(),
        score_weights: resolve_link_graph_rerank_score_weights(),
    }
}

#[cfg(test)]
mod tests;
