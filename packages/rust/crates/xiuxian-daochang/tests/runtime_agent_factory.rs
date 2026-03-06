//! Runtime-agent factory integration harness.

use xiuxian_daochang::{
    LITELLM_DEFAULT_URL, McpServerEntry, RuntimeSettings,
    test_support::{
        parse_embedding_backend_mode, resolve_inference_url,
        resolve_runtime_embedding_backend_mode, resolve_runtime_embedding_base_url,
        resolve_runtime_inference_url, resolve_runtime_memory_options, resolve_runtime_model,
        validate_inference_url_origin,
    },
};
use xiuxian_llm::embedding::backend::EmbeddingBackendKind as RuntimeEmbeddingBackendMode;

const _: fn(&RuntimeSettings) -> String = resolve_runtime_model;

#[path = "runtime_agent_factory/inference.rs"]
mod tests;
