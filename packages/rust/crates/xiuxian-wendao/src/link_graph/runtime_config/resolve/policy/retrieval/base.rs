use crate::link_graph::models::LinkGraphRetrievalMode;
use crate::link_graph::runtime_config::models::LinkGraphRetrievalPolicyRuntimeConfig;
use crate::link_graph::runtime_config::settings::{get_setting_string, merged_wendao_settings};
#[cfg(feature = "julia")]
use xiuxian_wendao_julia::compatibility::link_graph::LinkGraphJuliaRerankRuntimeConfig;
use xiuxian_wendao_runtime::runtime_config::resolve_link_graph_retrieval_base_runtime_with_settings;

/// Resolve retrieval policy runtime configuration from settings.
pub(crate) fn resolve_link_graph_retrieval_policy_runtime() -> LinkGraphRetrievalPolicyRuntimeConfig
{
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphRetrievalPolicyRuntimeConfig::from(
        resolve_link_graph_retrieval_base_runtime_with_settings(&settings),
    );

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.mode")
        .as_deref()
        .and_then(LinkGraphRetrievalMode::from_alias)
    {
        resolved.mode = value;
    }
    #[cfg(feature = "julia")]
    {
        resolved.julia_rerank = LinkGraphJuliaRerankRuntimeConfig::resolve_with_settings(&settings);
    }

    resolved
}
