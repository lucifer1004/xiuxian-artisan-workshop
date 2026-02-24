//! LLM runtime primitives.

/// Backend mode parsing and normalized backend kinds.
pub mod backend;
/// Core LLM client traits and HTTP implementations.
pub mod client;

pub use client::{ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmClient, OpenAIClient};
