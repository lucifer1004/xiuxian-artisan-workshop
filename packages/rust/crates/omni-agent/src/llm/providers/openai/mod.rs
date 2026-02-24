#[cfg(feature = "agent-provider-litellm")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "agent-provider-litellm")]
use litellm_rs::core::providers::openai::OpenAIProvider;

#[cfg(feature = "agent-provider-litellm")]
mod config;

#[cfg(feature = "agent-provider-litellm")]
use config::build_openai_config;

#[cfg(feature = "agent-provider-litellm")]
pub(in crate::llm) type LiteLlmOpenAIProvider = OpenAIProvider;
#[cfg(not(feature = "agent-provider-litellm"))]
#[cfg_attr(not(feature = "agent-provider-litellm"), allow(dead_code))]
pub(in crate::llm) type LiteLlmOpenAIProvider = ();

pub(in crate::llm) const DEFAULT_OPENAI_KEY_ENV: &str = "OPENAI_API_KEY";

#[cfg(feature = "agent-provider-litellm")]
pub(in crate::llm) async fn build_openai_provider(
    api_base: String,
    api_key: Option<String>,
    timeout_secs: u64,
) -> Result<LiteLlmOpenAIProvider> {
    let config = build_openai_config(api_base, api_key, timeout_secs);
    LiteLlmOpenAIProvider::new(config)
        .await
        .context("failed to initialize litellm-rs openai provider")
}

#[cfg(not(feature = "agent-provider-litellm"))]
#[cfg_attr(not(feature = "agent-provider-litellm"), allow(dead_code))]
pub(in crate::llm) async fn build_openai_provider(
    _api_base: String,
    _api_key: Option<String>,
    _timeout_secs: u64,
) -> Result<LiteLlmOpenAIProvider> {
    Err(anyhow::anyhow!(
        "litellm-rs backend is disabled at compile time (feature agent-provider-litellm)"
    ))
}
