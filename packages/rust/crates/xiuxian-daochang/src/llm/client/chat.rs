use anyhow::{Context, Result};
use litellm_rs::core::providers::base::BaseConfig;
use litellm_rs::core::providers::openai::{
    OpenAIProvider as LiteLlmOpenAIProvider, config::OpenAIConfig,
};
use litellm_rs::core::traits::provider::llm_provider::trait_definition::LLMProvider;
use litellm_rs::core::types::{
    ChatRequest as LiteChatRequest, RequestContext as LiteRequestContext,
    ToolChoice as LiteToolChoice,
};
use tokio::sync::OnceCell;

use super::backend::{LlmBackendMode, extract_api_base_from_inference_url, parse_backend_mode};
use super::converters::{
    chat_message_to_litellm_message, content_from_litellm, tool_call_from_litellm,
};
use super::tools::{PreparedTool, parse_tools_json};
use super::types::{AssistantMessage, ChatCompletionRequest, ChatCompletionResponse};
use crate::config::load_runtime_settings;
use crate::session::ChatMessage;

/// LLM client for chat completions.
pub struct LlmClient {
    client: reqwest::Client,
    inference_url: String,
    inference_api_base: String,
    model: String,
    api_key: Option<String>,
    backend_mode: LlmBackendMode,
    litellm_provider: OnceCell<LiteLlmOpenAIProvider>,
}

impl LlmClient {
    pub fn new(inference_url: String, model: String, api_key: Option<String>) -> Self {
        let env_backend = std::env::var("OMNI_AGENT_LLM_BACKEND")
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty());
        let (backend_mode, backend_source) = if let Some(raw) = env_backend.as_deref() {
            (parse_backend_mode(Some(raw)), "env")
        } else {
            let settings_backend = load_runtime_settings()
                .agent
                .llm_backend
                .map(|raw| raw.trim().to_string())
                .filter(|raw| !raw.is_empty());
            if let Some(raw) = settings_backend.as_deref() {
                (parse_backend_mode(Some(raw)), "settings")
            } else {
                (parse_backend_mode(None), "default")
            }
        };
        let inference_api_base = extract_api_base_from_inference_url(&inference_url);
        tracing::info!(
            llm_backend = backend_mode.as_str(),
            llm_backend_source = backend_source,
            inference_api_base = %inference_api_base,
            "llm backend selected"
        );
        Self {
            client: reqwest::Client::new(),
            inference_url,
            inference_api_base,
            model,
            api_key,
            backend_mode,
            litellm_provider: OnceCell::const_new(),
        }
    }

    /// Send messages and optionally tool definitions; returns content and/or tool_calls.
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools_json: Option<Vec<serde_json::Value>>,
    ) -> Result<AssistantMessage> {
        let tools = parse_tools_json(tools_json);
        match self.backend_mode {
            LlmBackendMode::OpenAiCompatibleHttp => self.chat_via_http(messages, tools).await,
            LlmBackendMode::LiteLlmRs => self.chat_via_litellm_rs(messages, tools).await,
        }
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
        };
        let mut req = self
            .client
            .post(&self.inference_url)
            .json(&body)
            .header("Content-Type", "application/json");
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let res = req.send().await?;
        let status = res.status();
        let text = res.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
        }
        let parsed: ChatCompletionResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("LLM response parse error: {}; body: {}", e, text))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("LLM response has no choices"))?;
        Ok(choice.message)
    }

    async fn chat_via_litellm_rs(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<PreparedTool>,
    ) -> Result<AssistantMessage> {
        let provider = self
            .litellm_provider
            .get_or_try_init(|| async {
                let api_key = self
                    .api_key
                    .clone()
                    .unwrap_or_else(|| "dummy-key-for-local".to_string());
                let config = OpenAIConfig {
                    base: BaseConfig {
                        api_key: Some(api_key),
                        api_base: Some(self.inference_api_base.clone()),
                        timeout: 60,
                        max_retries: 3,
                        headers: Default::default(),
                        organization: None,
                        api_version: None,
                    },
                    organization: None,
                    project: None,
                    model_mappings: Default::default(),
                    features: Default::default(),
                };
                LiteLlmOpenAIProvider::new(config)
                    .await
                    .context("failed to initialize litellm-rs openai provider")
            })
            .await?;

        let tools = if tools.is_empty() {
            None
        } else {
            Some(tools.iter().map(PreparedTool::to_litellm_tool).collect())
        };
        let request = LiteChatRequest {
            model: self.model.clone(),
            messages: messages
                .into_iter()
                .map(chat_message_to_litellm_message)
                .collect::<Result<Vec<_>>>()?,
            tools: tools.clone(),
            tool_choice: tools
                .as_ref()
                .map(|_| LiteToolChoice::String("auto".to_string())),
            ..Default::default()
        };

        let response = LLMProvider::chat_completion(provider, request, LiteRequestContext::new())
            .await
            .map_err(|e| anyhow::anyhow!("litellm-rs chat completion failed: {e}"))?;
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("litellm-rs response has no choices"))?;
        Ok(AssistantMessage {
            content: content_from_litellm(choice.message.content),
            tool_calls: choice
                .message
                .tool_calls
                .map(|calls| calls.into_iter().map(tool_call_from_litellm).collect()),
        })
    }
}
