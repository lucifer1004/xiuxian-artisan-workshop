use super::{
    LinkGraphSemanticIgnitionBackend, LinkGraphSemanticIgnitionRuntimeConfig,
    apply_semantic_ignition_runtime_config,
};
use crate::config::test_support;
use std::fs;

#[test]
fn semantic_ignition_runtime_reads_override_values() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.semantic_ignition]
backend = "openai-compatible"
vector_store_path = ".cache/store"
table_name = "docs"
embedding_base_url = "http://127.0.0.1:11434"
embedding_model = "glm-5"
"#,
    )?;

    let settings = test_support::load_test_settings_from_path(&config_path)?;
    let mut runtime = LinkGraphSemanticIgnitionRuntimeConfig::default();
    apply_semantic_ignition_runtime_config(&settings, &mut runtime);
    assert_eq!(
        runtime.backend,
        LinkGraphSemanticIgnitionBackend::OpenAiCompatible
    );
    assert_eq!(runtime.vector_store_path.as_deref(), Some(".cache/store"));
    assert_eq!(runtime.table_name.as_deref(), Some("docs"));
    assert_eq!(
        runtime.embedding_base_url.as_deref(),
        Some("http://127.0.0.1:11434")
    );
    assert_eq!(runtime.embedding_model.as_deref(), Some("glm-5"));

    Ok(())
}
