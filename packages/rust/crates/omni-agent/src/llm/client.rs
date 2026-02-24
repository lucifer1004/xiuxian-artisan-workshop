use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::Semaphore;

use super::backend::{LlmBackendMode, extract_api_base_from_inference_url, parse_backend_mode};
#[cfg(feature = "agent-provider-litellm")]
use super::compat::litellm::{LiteLlmDispatchConfig, LiteLlmRuntime};
use super::providers::{LiteLlmProviderMode, ProviderSettings, resolve_provider_settings};
use super::tools::{PreparedTool, parse_tools_json};
use super::types::{AssistantMessage, ChatCompletionRequest, ChatCompletionResponse};
use crate::config::load_runtime_settings;
use crate::session::ChatMessage;

/// LLM client for chat completions.
pub struct LlmClient {
    client: reqwest::Client,
    inference_url: String,
    #[cfg(feature = "agent-provider-litellm")]
    inference_api_base: String,
    model: String,
    api_key: Option<String>,
    backend_mode: LlmBackendMode,
    backend_source: &'static str,
    litellm_provider_mode: LiteLlmProviderMode,
    litellm_provider_source: &'static str,
    #[cfg(feature = "agent-provider-litellm")]
    litellm_api_key_env: String,
    #[cfg(feature = "agent-provider-litellm")]
    minimax_api_base: String,
    inference_timeout_secs: u64,
    inference_max_tokens: Option<u32>,
    inference_max_in_flight: Option<usize>,
    in_flight_gate: Option<Arc<Semaphore>>,
    #[cfg(feature = "agent-provider-litellm")]
    litellm_runtime: LiteLlmRuntime,
}

impl LlmClient {
    pub fn new(inference_url: String, model: String, api_key: Option<String>) -> Self {
        let runtime_settings = load_runtime_settings();
        let env_backend = std::env::var("OMNI_AGENT_LLM_BACKEND")
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty());
        let (backend_mode, backend_source) = if let Some(raw) = env_backend.as_deref() {
            (parse_backend_mode(Some(raw)), "env")
        } else {
            let settings_backend = runtime_settings
                .agent
                .llm_backend
                .as_deref()
                .map(str::trim)
                .map(ToString::to_string)
                .filter(|raw| !raw.is_empty());
            if let Some(raw) = settings_backend.as_deref() {
                (parse_backend_mode(Some(raw)), "settings")
            } else {
                (parse_backend_mode(None), "default")
            }
        };
        let provider_settings = resolve_provider_settings(&runtime_settings, model);
        let ProviderSettings {
            mode: litellm_provider_mode,
            source: litellm_provider_source,
            api_key_env: litellm_api_key_env,
            minimax_api_base,
            model,
            timeout_secs: inference_timeout_secs,
            max_tokens: inference_max_tokens,
            max_in_flight: inference_max_in_flight,
        } = provider_settings;
        let in_flight_gate = inference_max_in_flight.map(|limit| Arc::new(Semaphore::new(limit)));
        let inference_api_base = extract_api_base_from_inference_url(&inference_url);
        tracing::info!(
            llm_backend = backend_mode.as_str(),
            llm_backend_source = backend_source,
            litellm_provider = litellm_provider_mode.as_str(),
            litellm_provider_source = litellm_provider_source,
            litellm_api_key_env = %litellm_api_key_env,
            minimax_api_base = %minimax_api_base,
            inference_timeout_secs = inference_timeout_secs,
            inference_max_tokens = inference_max_tokens,
            inference_max_in_flight = inference_max_in_flight,
            model = %model,
            inference_api_base = %inference_api_base,
            "llm backend selected"
        );
        Self {
            client: build_http_client(),
            inference_url,
            #[cfg(feature = "agent-provider-litellm")]
            inference_api_base,
            model,
            api_key,
            backend_mode,
            backend_source,
            litellm_provider_mode,
            litellm_provider_source,
            #[cfg(feature = "agent-provider-litellm")]
            litellm_api_key_env,
            #[cfg(feature = "agent-provider-litellm")]
            minimax_api_base,
            inference_timeout_secs,
            inference_max_tokens,
            inference_max_in_flight,
            in_flight_gate,
            #[cfg(feature = "agent-provider-litellm")]
            litellm_runtime: LiteLlmRuntime::new(),
        }
    }

    /// Active backend mode label (`litellm_rs` or `http`).
    pub fn backend_mode(&self) -> &'static str {
        self.backend_mode.as_str()
    }

    /// Backend source label (`env`, `settings`, or `default`).
    pub fn backend_source(&self) -> &'static str {
        self.backend_source
    }

    /// Active litellm provider mode (`openai` or `minimax`).
    pub fn litellm_provider_mode(&self) -> &'static str {
        self.litellm_provider_mode.as_str()
    }

    /// litellm provider source (`env`, `settings`, `default`).
    pub fn litellm_provider_source(&self) -> &'static str {
        self.litellm_provider_source
    }

    /// Send messages and optionally tool definitions; returns content and/or `tool_calls`.
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools_json: Option<Vec<serde_json::Value>>,
    ) -> Result<AssistantMessage> {
        let tools = parse_tools_json(tools_json);
        let started_at = Instant::now();
        let gate_wait_started = Instant::now();
        let _in_flight_permit = if let Some(gate) = self.in_flight_gate.as_ref() {
            Some(
                gate.clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| anyhow::anyhow!("llm in-flight gate closed unexpectedly"))?,
            )
        } else {
            None
        };
        let gate_wait_ms =
            u64::try_from(gate_wait_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        tracing::debug!(
            event = "agent.llm.chat.dispatch",
            llm_backend = self.backend_mode(),
            llm_backend_source = self.backend_source(),
            litellm_provider = self.litellm_provider_mode(),
            litellm_provider_source = self.litellm_provider_source(),
            inference_max_in_flight = self.inference_max_in_flight,
            gate_wait_ms = gate_wait_ms,
            message_count = messages.len(),
            tools_count = tools.len(),
            "dispatching llm chat request"
        );
        let result = match self.backend_mode {
            LlmBackendMode::OpenAiCompatibleHttp => self.chat_via_http(messages, tools).await,
            LlmBackendMode::LiteLlmRs => {
                #[cfg(feature = "agent-provider-litellm")]
                {
                    self.chat_via_litellm_rs(messages, tools).await
                }
                #[cfg(not(feature = "agent-provider-litellm"))]
                {
                    let _ = (messages, tools);
                    Err(anyhow::anyhow!(
                        "litellm-rs backend is disabled at compile time (feature agent-provider-litellm)"
                    ))
                }
            }
        };
        let elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        match &result {
            Ok(message) => {
                let tool_call_count = message.tool_calls.as_ref().map_or(0, std::vec::Vec::len);
                tracing::debug!(
                    event = "agent.llm.chat.completed",
                    llm_backend = self.backend_mode(),
                    litellm_provider = self.litellm_provider_mode(),
                    elapsed_ms = elapsed_ms,
                    tool_call_count = tool_call_count,
                    "llm chat request completed"
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "agent.llm.chat.failed",
                    llm_backend = self.backend_mode(),
                    litellm_provider = self.litellm_provider_mode(),
                    elapsed_ms = elapsed_ms,
                    error = %error,
                    "llm chat request failed"
                );
            }
        }
        result
    }

    async fn chat_via_http(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<PreparedTool>,
    ) -> Result<AssistantMessage> {
        let tools = if tools.is_empty() {
            None
        } else {
            Some(tools.iter().map(PreparedTool::to_http_tool_def).collect())
        };
        let body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tool_choice: tools.as_ref().map(|_| "auto".to_string()),
            tools,
            max_tokens: self.inference_max_tokens,
        };
        let mut req = self
            .client
            .post(&self.inference_url)
            .json(&body)
            .header("Content-Type", "application/json");
        req = req.timeout(Duration::from_secs(self.inference_timeout_secs));
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await?;
        let status = res.status();
        let text = res.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("LLM API error {status}: {text}"));
        }
        let parsed: ChatCompletionResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("LLM response parse error: {e}; body: {text}"))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("LLM response has no choices"))?;
        Ok(choice.message)
    }

    #[cfg(feature = "agent-provider-litellm")]
    async fn chat_via_litellm_rs(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<PreparedTool>,
    ) -> Result<AssistantMessage> {
        self.litellm_runtime
            .chat(
                LiteLlmDispatchConfig {
                    provider_mode: self.litellm_provider_mode,
                    model: &self.model,
                    max_tokens: self.inference_max_tokens,
                    api_key: self.api_key.as_deref(),
                    litellm_api_key_env: &self.litellm_api_key_env,
                    inference_api_base: &self.inference_api_base,
                    minimax_api_base: &self.minimax_api_base,
                    timeout_secs: self.inference_timeout_secs,
                },
                messages,
                tools,
            )
            .await
    }
}

fn build_http_client() -> reqwest::Client {
    let builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(64)
        .tcp_nodelay(true);
    match builder.build() {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to build tuned llm http client; falling back to default client"
            );
            reqwest::Client::new()
        }
    }
}
