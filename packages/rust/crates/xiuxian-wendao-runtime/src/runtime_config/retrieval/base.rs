use crate::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER, DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
    DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE, DEFAULT_LINK_GRAPH_MAX_SOURCES,
    DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE, LINK_GRAPH_CANDIDATE_MULTIPLIER_ENV,
    LINK_GRAPH_HYBRID_MIN_HITS_ENV, LINK_GRAPH_HYBRID_MIN_TOP_SCORE_ENV,
    LINK_GRAPH_MAX_SOURCES_ENV, LINK_GRAPH_ROWS_PER_SOURCE_ENV,
};
use crate::settings::{
    first_non_empty, get_setting_string, parse_positive_f64, parse_positive_usize,
};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use super::semantic_ignition::{
    LinkGraphSemanticIgnitionRuntimeConfig, apply_semantic_ignition_runtime_config,
};

/// Generic retrieval tuning owned by the Wendao runtime host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkGraphRetrievalBaseRuntimeConfig {
    /// Candidate multiplier applied to the effective search limit.
    pub candidate_multiplier: usize,
    /// Maximum distinct source hints forwarded into later stages.
    pub max_sources: usize,
    /// Minimum graph hit count required for graph sufficiency.
    pub hybrid_min_hits: usize,
    /// Minimum top graph score required for graph sufficiency.
    pub hybrid_min_top_score: f64,
    /// Maximum graph rows requested per source hint.
    pub graph_rows_per_source: usize,
    /// Semantic ignition runtime knobs.
    pub semantic_ignition: LinkGraphSemanticIgnitionRuntimeConfig,
}

impl Default for LinkGraphRetrievalBaseRuntimeConfig {
    fn default() -> Self {
        Self {
            candidate_multiplier: DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER,
            max_sources: DEFAULT_LINK_GRAPH_MAX_SOURCES,
            hybrid_min_hits: DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
            hybrid_min_top_score: DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE,
            graph_rows_per_source: DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE,
            semantic_ignition: LinkGraphSemanticIgnitionRuntimeConfig::default(),
        }
    }
}

/// Resolve generic retrieval tuning from merged Wendao settings.
pub fn resolve_link_graph_retrieval_base_runtime_with_settings(
    settings: &Value,
) -> LinkGraphRetrievalBaseRuntimeConfig {
    let mut resolved = LinkGraphRetrievalBaseRuntimeConfig::default();

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.candidate_multiplier"),
        std::env::var(LINK_GRAPH_CANDIDATE_MULTIPLIER_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.candidate_multiplier = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.max_sources"),
        std::env::var(LINK_GRAPH_MAX_SOURCES_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_sources = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.hybrid_min_hits"),
        std::env::var(LINK_GRAPH_HYBRID_MIN_HITS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.hybrid_min_hits = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.hybrid_min_top_score"),
        std::env::var(LINK_GRAPH_HYBRID_MIN_TOP_SCORE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64)
    {
        resolved.hybrid_min_top_score = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.graph_rows_per_source"),
        std::env::var(LINK_GRAPH_ROWS_PER_SOURCE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.graph_rows_per_source = value;
    }

    apply_semantic_ignition_runtime_config(settings, &mut resolved.semantic_ignition);

    resolved
}

#[cfg(test)]
mod tests {
    use super::resolve_link_graph_retrieval_base_runtime_with_settings;
    use crate::runtime_config::{LinkGraphSemanticIgnitionBackend, LinkGraphSemanticIgnitionRuntimeConfig};
    use crate::settings::{merged_toml_settings, set_link_graph_wendao_config_override};
    use serial_test::serial;
    use std::fs;

    #[test]
    #[serial]
    fn retrieval_base_runtime_reads_override_values() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.retrieval]
candidate_multiplier = 3
max_sources = 5
hybrid_min_hits = 4
hybrid_min_top_score = 0.6
graph_rows_per_source = 9

[link_graph.retrieval.semantic_ignition]
backend = "openai-compatible"
embedding_model = "glm-5"
"#,
        )?;
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let settings = merged_toml_settings("link_graph", "", "", "wendao.toml");
        let runtime = resolve_link_graph_retrieval_base_runtime_with_settings(&settings);
        assert_eq!(runtime.candidate_multiplier, 3);
        assert_eq!(runtime.max_sources, 5);
        assert_eq!(runtime.hybrid_min_hits, 4);
        assert_eq!(runtime.hybrid_min_top_score, 0.6);
        assert_eq!(runtime.graph_rows_per_source, 9);
        assert_eq!(
            runtime.semantic_ignition,
            LinkGraphSemanticIgnitionRuntimeConfig {
                backend: LinkGraphSemanticIgnitionBackend::OpenAiCompatible,
                embedding_model: Some("glm-5".to_string()),
                ..LinkGraphSemanticIgnitionRuntimeConfig::default()
            }
        );

        Ok(())
    }
}
