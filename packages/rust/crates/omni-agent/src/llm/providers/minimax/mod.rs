#[cfg(feature = "agent-provider-litellm")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "agent-provider-litellm")]
use litellm_rs::core::providers::minimax::{MinimaxConfig, MinimaxProvider};

mod normalize;

pub(in crate::llm) use normalize::{normalize_minimax_api_base, normalize_minimax_model};

#[cfg(feature = "agent-provider-litellm")]
pub(in crate::llm) type LiteLlmMinimaxProvider = MinimaxProvider;
#[cfg(not(feature = "agent-provider-litellm"))]
#[cfg_attr(not(feature = "agent-provider-litellm"), allow(dead_code))]
pub(in crate::llm) type LiteLlmMinimaxProvider = ();

pub(in crate::llm) const DEFAULT_MINIMAX_KEY_ENV: &str = "MINIMAX_API_KEY";

#[cfg(feature = "agent-provider-litellm")]
pub(in crate::llm) async fn build_minimax_provider(
    api_base: String,
    api_key: String,
    timeout_secs: u64,
) -> Result<LiteLlmMinimaxProvider> {
    let config = MinimaxConfig {
        api_key,
        api_base,
        timeout_seconds: timeout_secs,
        max_retries: 3,
    };
    LiteLlmMinimaxProvider::new(config)
        .await
        .context("failed to initialize litellm-rs minimax provider")
}

#[cfg(not(feature = "agent-provider-litellm"))]
#[cfg_attr(not(feature = "agent-provider-litellm"), allow(dead_code))]
pub(in crate::llm) async fn build_minimax_provider(
    _api_base: String,
    _api_key: String,
    _timeout_secs: u64,
) -> Result<LiteLlmMinimaxProvider> {
    Err(anyhow::anyhow!(
        "litellm-rs backend is disabled at compile time (feature agent-provider-litellm)"
    ))
}
