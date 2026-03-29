use super::types::{AgentConfig, ContextBudgetStrategy};

pub(super) fn default_max_tool_rounds() -> u32 {
    30
}

pub(super) fn default_tool_pool_size() -> usize {
    4
}

pub(super) fn default_tool_handshake_timeout_secs() -> u64 {
    30
}

pub(super) fn default_tool_connect_retries() -> u32 {
    3
}

pub(super) fn default_tool_strict_startup() -> bool {
    true
}

pub(super) fn default_tool_connect_retry_backoff_ms() -> u64 {
    1_000
}

pub(super) fn default_tool_timeout_secs() -> u64 {
    180
}

pub(super) fn default_tool_list_cache_ttl_ms() -> u64 {
    1_000
}

pub(super) fn default_consolidation_take_turns() -> usize {
    10
}

pub(super) fn default_consolidation_async() -> bool {
    true
}

pub(super) fn default_context_budget_reserve_tokens() -> usize {
    512
}

pub(super) fn default_summary_max_segments() -> usize {
    8
}

pub(super) fn default_summary_max_chars() -> usize {
    480
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            inference_url: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            tool_servers: Vec::new(),
            tool_pool_size: default_tool_pool_size(),
            tool_handshake_timeout_secs: default_tool_handshake_timeout_secs(),
            tool_connect_retries: default_tool_connect_retries(),
            tool_strict_startup: default_tool_strict_startup(),
            tool_connect_retry_backoff_ms: default_tool_connect_retry_backoff_ms(),
            tool_timeout_secs: default_tool_timeout_secs(),
            tool_list_cache_ttl_ms: default_tool_list_cache_ttl_ms(),
            max_tool_rounds: default_max_tool_rounds(),
            memory: None,
            window_max_turns: None,
            consolidation_threshold_turns: None,
            consolidation_take_turns: default_consolidation_take_turns(),
            consolidation_async: default_consolidation_async(),
            context_budget_tokens: None,
            context_budget_reserve_tokens: default_context_budget_reserve_tokens(),
            context_budget_strategy: ContextBudgetStrategy::default(),
            summary_max_segments: default_summary_max_segments(),
            summary_max_chars: default_summary_max_chars(),
        }
    }
}

/// Default `LiteLLM` proxy path (when using `litellm --port 4000`).
pub const LITELLM_DEFAULT_URL: &str = "http://localhost:4000/v1/chat/completions";

impl AgentConfig {
    /// Build config that uses a `LiteLLM` proxy as the inference endpoint.
    pub fn litellm(model: impl Into<String>) -> Self {
        let inference_url =
            std::env::var("LITELLM_PROXY_URL").unwrap_or_else(|_| LITELLM_DEFAULT_URL.to_string());
        let model = std::env::var("OMNI_AGENT_MODEL").unwrap_or_else(|_| model.into());
        Self {
            inference_url,
            model,
            api_key: None,
            tool_servers: Vec::new(),
            tool_pool_size: default_tool_pool_size(),
            tool_handshake_timeout_secs: default_tool_handshake_timeout_secs(),
            tool_connect_retries: default_tool_connect_retries(),
            tool_strict_startup: default_tool_strict_startup(),
            tool_connect_retry_backoff_ms: default_tool_connect_retry_backoff_ms(),
            tool_timeout_secs: default_tool_timeout_secs(),
            tool_list_cache_ttl_ms: default_tool_list_cache_ttl_ms(),
            max_tool_rounds: default_max_tool_rounds(),
            memory: None,
            window_max_turns: None,
            consolidation_threshold_turns: None,
            consolidation_take_turns: default_consolidation_take_turns(),
            consolidation_async: default_consolidation_async(),
            context_budget_tokens: None,
            context_budget_reserve_tokens: default_context_budget_reserve_tokens(),
            context_budget_strategy: ContextBudgetStrategy::default(),
            summary_max_segments: default_summary_max_segments(),
            summary_max_chars: default_summary_max_chars(),
        }
    }

    /// Resolve API key: config value, or env (`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`).
    /// When inference goes to our own loopback tool/inference gateway, returns None
    /// so we do not send a key — the local service holds the key and forwards to the real LLM.
    #[must_use]
    pub fn resolve_api_key(&self) -> Option<String> {
        self.resolve_api_key_with_env_reader(|key| std::env::var(key).ok())
    }

    /// Resolve API key using a pluggable environment reader.
    ///
    /// This keeps runtime behavior identical while allowing deterministic tests
    /// without mutating process-wide environment variables.
    #[must_use]
    pub fn resolve_api_key_with_env_reader<F>(&self, mut read_env: F) -> Option<String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        if let Some(ref key) = self.api_key {
            return Some(key.clone());
        }
        if self.inference_url.contains("127.0.0.1") || self.inference_url.contains("localhost") {
            return None;
        }
        if self.inference_url.contains("anthropic")
            || self.inference_url.contains("claude")
            || self.inference_url.contains("/messages")
        {
            return read_env("ANTHROPIC_API_KEY").or_else(|| read_env("ANTHROPIC_AUTH_TOKEN"));
        }
        read_env("OPENAI_API_KEY")
    }
}
