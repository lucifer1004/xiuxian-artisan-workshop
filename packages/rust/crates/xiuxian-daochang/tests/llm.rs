//! LLM module integration harness.

#[path = "llm/backend.rs"]
mod backend_tests;
#[cfg(feature = "agent-provider-litellm")]
#[path = "llm/converters_multimodal.rs"]
mod converters_multimodal_tests;
#[path = "llm/http_request.rs"]
mod http_request_tests;
#[cfg(feature = "agent-provider-litellm")]
#[path = "llm/litellm_custom_base_keys.rs"]
mod litellm_custom_base_keys_tests;
#[path = "llm/message_integrity.rs"]
mod message_integrity_tests;
#[path = "llm/provider_mode.rs"]
mod provider_mode_tests;
