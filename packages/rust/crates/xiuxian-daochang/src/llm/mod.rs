//! LLM namespace: backend selection, request mapping, and chat client.

mod backend;
mod client;
mod compat;
#[cfg(feature = "agent-provider-litellm")]
mod converters;
mod protocol;
mod providers;
pub(crate) mod test_api;
mod tools;
mod types;

pub use client::{LlmClient, LlmInFlightSnapshot};
pub use types::AssistantMessage;
