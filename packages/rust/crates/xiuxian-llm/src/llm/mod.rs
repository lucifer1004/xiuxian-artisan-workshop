//! LLM runtime primitives.

/// Unified acceleration mode parsing and config resolution.
pub mod acceleration;
/// Backend mode parsing and normalized backend kinds.
pub mod backend;
/// Core LLM client traits and HTTP implementations.
pub mod client;
/// Structured LLM error model with user-safe sanitization.
pub mod error;
/// Platform-agnostic multimodal marker parsing utilities.
pub mod multimodal;
/// Provider builders shared by runtime facades.
pub mod providers;
/// Vision preprocessing and semantic grounding utilities.
pub mod vision;

pub use client::{
    ChatChoice, ChatMessage, ChatRequest, ChatResponse, ContentPart, ImageUrlContent, LlmClient,
    MessageContent, OpenAIClient,
};
pub use error::{LlmError, LlmResult};
