//! Embedding module integration tests.

use xiuxian_daochang::test_support::embed_http;

#[cfg(feature = "agent-provider-litellm")]
use xiuxian_daochang::test_support::{
    OLLAMA_PLACEHOLDER_API_KEY, normalize_litellm_embedding_target,
    normalize_openai_compatible_base_url,
};

#[path = "embedding/backend.rs"]
mod backend;

#[cfg(feature = "agent-provider-litellm")]
#[path = "embedding/transport_litellm.rs"]
mod transport_litellm;

#[path = "embedding/transport_openai.rs"]
mod transport_openai;

#[path = "embedding/transport_http.rs"]
mod transport_http;
