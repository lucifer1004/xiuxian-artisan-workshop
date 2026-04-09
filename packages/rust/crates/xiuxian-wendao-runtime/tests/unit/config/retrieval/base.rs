use super::resolve_link_graph_retrieval_base_runtime_with_settings;
use crate::config::{
    LinkGraphSemanticIgnitionBackend, LinkGraphSemanticIgnitionRuntimeConfig, test_support,
};
use std::fs;

#[test]
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

    let settings = test_support::load_test_settings_from_path(&config_path)?;
    let runtime = resolve_link_graph_retrieval_base_runtime_with_settings(&settings);
    assert_eq!(runtime.candidate_multiplier, 3);
    assert_eq!(runtime.max_sources, 5);
    assert_eq!(runtime.hybrid_min_hits, 4);
    assert!((runtime.hybrid_min_top_score - 0.6).abs() < 1e-12);
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
