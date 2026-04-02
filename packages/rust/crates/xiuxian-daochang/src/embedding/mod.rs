//! Embedding client runtime.
//!
//! Supports three backends:
//! - `http`: `/embed/batch` HTTP transport.
//! - `openai_http`: generic OpenAI-compatible `/v1/embeddings`.
//! - `litellm_rs`: Rust-native `LiteLLM` provider path (provider/API-key driven).

mod backend;
mod cache;
mod client;
mod transport_http;
#[cfg(feature = "agent-provider-litellm")]
mod transport_litellm;
mod transport_openai;
mod types;

pub use client::{EmbeddingClient, EmbeddingInFlightSnapshot};

pub(crate) fn test_parse_backend_mode(
    raw: Option<&str>,
) -> xiuxian_llm::embedding::backend::EmbeddingBackendKind {
    backend::test_parse_backend_mode(raw)
}

#[cfg(feature = "agent-provider-litellm")]
pub(crate) const TEST_OLLAMA_PLACEHOLDER_API_KEY: &str =
    transport_litellm::TEST_OLLAMA_PLACEHOLDER_API_KEY;

#[cfg(feature = "agent-provider-litellm")]
pub(crate) fn test_normalize_openai_compatible_base_url(api_base: &str) -> String {
    transport_litellm::test_normalize_openai_compatible_base_url(api_base)
}

#[cfg(feature = "agent-provider-litellm")]
pub(crate) fn test_normalize_litellm_embedding_target(
    model: &str,
    api_base: &str,
    api_key: Option<&str>,
) -> (String, String, Option<String>, bool) {
    transport_litellm::test_normalize_litellm_embedding_target(model, api_base, api_key)
}

pub(crate) async fn test_embed_http(
    client: &reqwest::Client,
    base_url: &str,
    texts: &[String],
    model: Option<&str>,
) -> Option<Vec<Vec<f32>>> {
    transport_http::embed_http(client, base_url, texts, model).await
}
