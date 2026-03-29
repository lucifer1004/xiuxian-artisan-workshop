use crate::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_SEMANTIC_IGNITION_BACKEND, LINK_GRAPH_SEMANTIC_IGNITION_BACKEND_ENV,
    LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_BASE_URL_ENV,
    LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_MODEL_ENV, LINK_GRAPH_SEMANTIC_IGNITION_TABLE_NAME_ENV,
    LINK_GRAPH_SEMANTIC_IGNITION_VECTOR_STORE_PATH_ENV,
};
use crate::settings::{first_non_empty, get_setting_string};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// Runtime-selectable backend for semantic ignition enrichment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkGraphSemanticIgnitionBackend {
    /// Disable semantic ignition enrichment.
    #[default]
    Disabled,
    /// Use precomputed vectors with the Rust vector store.
    VectorStore,
    /// Resolve embeddings through an OpenAI-compatible endpoint, then query the vector store.
    OpenAiCompatible,
}

impl LinkGraphSemanticIgnitionBackend {
    /// Parse stable aliases used in runtime configuration.
    #[must_use]
    pub fn from_alias(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "off" | "none" | "disabled" => Some(Self::Disabled),
            "vector" | "vector_store" | "vector-store" | "xiuxian_vector" | "xiuxian-vector" => {
                Some(Self::VectorStore)
            }
            "openai" | "openai_compatible" | "openai-compatible" | "glm" | "glm_openai" => {
                Some(Self::OpenAiCompatible)
            }
            _ => None,
        }
    }
}

/// Runtime knobs for semantic ignition enrichment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraphSemanticIgnitionRuntimeConfig {
    /// Selected semantic ignition backend.
    pub backend: LinkGraphSemanticIgnitionBackend,
    /// Base path used to open the vector store.
    pub vector_store_path: Option<String>,
    /// Logical table name within the vector store.
    pub table_name: Option<String>,
    /// OpenAI-compatible embeddings base URL.
    pub embedding_base_url: Option<String>,
    /// Optional embedding model name.
    pub embedding_model: Option<String>,
}

impl Default for LinkGraphSemanticIgnitionRuntimeConfig {
    fn default() -> Self {
        Self {
            backend: LinkGraphSemanticIgnitionBackend::from_alias(
                DEFAULT_LINK_GRAPH_SEMANTIC_IGNITION_BACKEND,
            )
            .unwrap_or_default(),
            vector_store_path: None,
            table_name: None,
            embedding_base_url: None,
            embedding_model: None,
        }
    }
}

/// Apply semantic-ignition runtime settings from merged Wendao configuration.
pub fn apply_semantic_ignition_runtime_config(
    settings: &Value,
    resolved: &mut LinkGraphSemanticIgnitionRuntimeConfig,
) {
    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.semantic_ignition.backend"),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_BACKEND_ENV).ok(),
    ])
    .as_deref()
    .and_then(LinkGraphSemanticIgnitionBackend::from_alias)
    {
        resolved.backend = value;
    }

    resolved.vector_store_path = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.semantic_ignition.vector_store_path",
        ),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_VECTOR_STORE_PATH_ENV).ok(),
    ]));
    resolved.table_name = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.semantic_ignition.table_name",
        ),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_TABLE_NAME_ENV).ok(),
    ]));
    resolved.embedding_base_url = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.semantic_ignition.embedding_base_url",
        ),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_BASE_URL_ENV).ok(),
    ]));
    resolved.embedding_model = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.semantic_ignition.embedding_model",
        ),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_MODEL_ENV).ok(),
    ]));
}

fn normalize_optional_runtime_string(value: Option<String>) -> Option<String> {
    value
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        LinkGraphSemanticIgnitionBackend, LinkGraphSemanticIgnitionRuntimeConfig,
        apply_semantic_ignition_runtime_config,
    };
    use crate::settings::{merged_toml_settings, set_link_graph_wendao_config_override};
    use serial_test::serial;
    use std::fs;

    #[test]
    #[serial]
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
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let settings = merged_toml_settings("link_graph", "", "", "wendao.toml");
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
}
