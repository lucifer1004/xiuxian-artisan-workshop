use crate::link_graph::models::LinkGraphRetrievalMode;
use crate::link_graph::runtime_config::models::LinkGraphRetrievalPolicyRuntimeConfig;
use crate::link_graph::runtime_config::settings::{get_setting_string, merged_wendao_settings};
use xiuxian_wendao_builtin::resolve_builtin_rerank_runtime_projection_with_settings;
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
    let rerank = resolve_builtin_rerank_runtime_projection_with_settings(&settings);
    resolved.rerank_binding = rerank.binding;
    resolved.rerank_schema_version = rerank.schema_version;
    resolved.rerank_score_weights = rerank.score_weights;

    resolved
}
