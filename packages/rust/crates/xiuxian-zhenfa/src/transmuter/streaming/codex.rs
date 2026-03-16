//! Codex / OpenAI streaming parser.
//!
//! Parses SSE events from OpenAI-compatible APIs, mapping native events
//! to the unified `ZhenfaStreamingEvent` model.

use super::ZhenfaStreamingEvent;
use super::traits::{StreamingOutcome, StreamingTransmuter, TokenUsage};
use serde::Deserialize;

/// OpenAI streaming response structure.
#[derive(Debug, Clone, Deserialize)]
struct CodexResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
struct Choice {
    delta: Option<Delta>,
    message: Option<Message>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Delta {
    content: Option<String>,
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Clone, Deserialize)]
struct ToolCallDelta {
    index: u32,
    id: Option<String>,
    function: Option<FunctionDelta>,
}

#[derive(Debug, Clone, Deserialize)]
struct FunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Deserialize)]
struct ToolCall {
    id: String,
    function: FunctionCall,
}

#[derive(Debug, Clone, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

/// Parser for Codex / OpenAI-style streaming output.
#[derive(Debug, Default)]
pub struct CodexStreamingParser {
    accumulated: String,
    tool_call_buffers: Vec<(Option<String>, Option<String>, String)>, // (id, name, args)
    final_usage: Option<TokenUsage>,
    finish_reason: Option<String>,
}

impl CodexStreamingParser {
    /// Create a new Codex streaming parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StreamingTransmuter for CodexStreamingParser {
    fn parse_line(&mut self, line: &str) -> Result<Vec<ZhenfaStreamingEvent>, String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(Vec::new());
        }

        // Handle SSE format
        let json_str = if let Some(stripped) = line.strip_prefix("data: ") {
            stripped
        } else {
            line
        };

        // Check for stream end
        if json_str == "[DONE]" {
            let outcome = StreamingOutcome {
                success: true,
                tokens_used: self.final_usage,
                final_text: std::mem::take(&mut self.accumulated).into(),
                tool_calls: Vec::new(),
                exit_code: Some(0),
            };
            return Ok(vec![ZhenfaStreamingEvent::Finished(outcome)]);
        }

        let response: CodexResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse Codex response: {e}"))?;

        // Extract usage if present
        if let Some(usage) = response.usage {
            self.final_usage = Some(TokenUsage {
                input: usage.prompt_tokens,
                output: usage.completion_tokens,
                total: usage.total_tokens,
            });
        }

        let mut events = Vec::new();

        // Process choices
        for choice in response.choices {
            if let Some(reason) = &choice.finish_reason {
                self.finish_reason = Some(reason.clone());
            }

            if let Some(delta) = &choice.delta {
                // Handle content
                if let Some(content) = &delta.content {
                    self.accumulated.push_str(content);
                    events.push(ZhenfaStreamingEvent::TextDelta(content.clone().into()));
                }

                // Handle reasoning/thinking
                if let Some(reasoning) = &delta.reasoning_content {
                    events.push(ZhenfaStreamingEvent::Thought(reasoning.clone().into()));
                }

                // Handle tool calls
                for tc in &delta.tool_calls {
                    // Ensure buffer exists
                    while self.tool_call_buffers.len() <= tc.index as usize {
                        self.tool_call_buffers.push((None, None, String::new()));
                    }

                    let buffer = &mut self.tool_call_buffers[tc.index as usize];

                    if let Some(id) = &tc.id {
                        buffer.0 = Some(id.clone());
                    }

                    if let Some(func) = &tc.function {
                        if let Some(name) = &func.name {
                            buffer.1 = Some(name.clone());
                        }
                        if let Some(args) = &func.arguments {
                            buffer.2.push_str(args);
                        }
                    }
                }
            }

            // Handle complete message (non-streaming)
            if let Some(message) = &choice.message {
                if let Some(content) = &message.content {
                    self.accumulated.push_str(content);
                }

                for tc in &message.tool_calls {
                    let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null);
                    events.push(ZhenfaStreamingEvent::ToolCall {
                        id: tc.id.clone().into(),
                        name: tc.function.name.clone().into(),
                        input: args,
                    });
                }
            }
        }

        Ok(events)
    }

    fn finalize(&mut self) -> Result<Option<ZhenfaStreamingEvent>, String> {
        // Emit any pending tool calls
        for (id, name, args) in std::mem::take(&mut self.tool_call_buffers) {
            if let (Some(id), Some(name)) = (id, name) {
                let input: serde_json::Value =
                    serde_json::from_str(&args).unwrap_or(serde_json::Value::Null);
                // Return first tool call, store rest for later
                return Ok(Some(ZhenfaStreamingEvent::ToolCall {
                    id: id.into(),
                    name: name.into(),
                    input,
                }));
            }
        }

        if !self.accumulated.is_empty() || self.final_usage.is_some() {
            let outcome = StreamingOutcome {
                success: true,
                tokens_used: self.final_usage.take(),
                final_text: std::mem::take(&mut self.accumulated).into(),
                tool_calls: Vec::new(),
                exit_code: Some(0),
            };
            Ok(Some(ZhenfaStreamingEvent::Finished(outcome)))
        } else {
            Ok(None)
        }
    }

    fn accumulated_text(&self) -> &str {
        &self.accumulated
    }

    fn reset(&mut self) {
        self.accumulated.clear();
        self.tool_call_buffers.clear();
        self.final_usage = None;
        self.finish_reason = None;
    }

    fn provider_name(&self) -> &'static str {
        "codex"
    }
}
