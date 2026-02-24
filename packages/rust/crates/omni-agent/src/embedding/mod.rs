//! Embedding client runtime.
//!
//! Supports three backends:
//! - `http`: legacy `/embed/batch` with optional MCP fallback.
//! - `openai_http`: OpenAI-compatible `/v1/embeddings` (for example `mistralrs serve`).
//! - `litellm_rs`: Rust-native `LiteLLM` embedding path.

mod backend;
mod cache;
mod client;
mod transport_http;
#[cfg(feature = "agent-provider-litellm")]
mod transport_litellm;
mod transport_mcp;
mod transport_openai;
mod types;

pub use client::EmbeddingClient;
