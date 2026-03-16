//! Claude Code CLI streaming parser.
//!
//! Parses NDJSON output from Claude Code CLI, mapping native events
//! to the unified `ZhenfaStreamingEvent` model.

use super::ZhenfaStreamingEvent;
use super::traits::{StreamingOutcome, StreamingTransmuter, TokenUsage};
use serde::Deserialize;

/// NDJSON event types from Claude Code CLI.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum ClaudeEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: ClaudeMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDelta,
        usage: Option<UsageInfo>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: ErrorInfo },
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeMessage {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    role: String,
    #[allow(dead_code)]
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[allow(dead_code)]
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Clone, Deserialize)]
struct MessageDelta {
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageInfo {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ErrorInfo {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

/// Parser for Claude Code CLI streaming output.
#[derive(Debug, Default)]
pub struct ClaudeStreamingParser {
    accumulated: String,
    tool_input_buffers: Vec<(String, String, String)>, // (id, name, partial_json)
    final_usage: Option<TokenUsage>,
    stop_reason: Option<String>,
}

impl ClaudeStreamingParser {
    /// Create a new Claude streaming parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StreamingTransmuter for ClaudeStreamingParser {
    fn parse_line(&mut self, line: &str) -> Result<Vec<ZhenfaStreamingEvent>, String> {
        let line = line.trim();
        if line.is_empty() || line == "data: [DONE]" {
            return Ok(Vec::new());
        }

        let json_str = line.strip_prefix("data: ").unwrap_or(line);

        let event: ClaudeEvent = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse Claude event: {e}"))?;

        match event {
            ClaudeEvent::MessageStart { .. } => Ok(Vec::new()),
            ClaudeEvent::ContentBlockStart { content_block, .. } => {
                if let ContentBlock::ToolUse { id, name, .. } = content_block {
                    self.tool_input_buffers.push((id, name, String::new()));
                }
                Ok(Vec::new())
            }
            ClaudeEvent::ContentBlockDelta { index, delta } => match delta {
                ContentDelta::TextDelta { text } => {
                    self.accumulated.push_str(&text);
                    Ok(vec![ZhenfaStreamingEvent::TextDelta(text.into())])
                }
                ContentDelta::ThinkingDelta { thinking } => {
                    Ok(vec![ZhenfaStreamingEvent::Thought(thinking.into())])
                }
                ContentDelta::InputJsonDelta { partial_json } => {
                    if let Some((_, _, buf)) = self.tool_input_buffers.get_mut(index as usize) {
                        buf.push_str(&partial_json);
                    }
                    Ok(Vec::new())
                }
            },
            ClaudeEvent::ContentBlockStop { index } => {
                if let Some((id, name, input)) =
                    self.tool_input_buffers.get(index as usize).cloned()
                {
                    let parsed_input: serde_json::Value =
                        serde_json::from_str(&input).unwrap_or(serde_json::Value::Null);
                    return Ok(vec![ZhenfaStreamingEvent::ToolCall {
                        id: id.into(),
                        name: name.into(),
                        input: parsed_input,
                    }]);
                }
                Ok(Vec::new())
            }
            ClaudeEvent::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason;
                if let Some(u) = usage {
                    self.final_usage = Some(TokenUsage::new(u.input_tokens, u.output_tokens));
                }
                Ok(Vec::new())
            }
            ClaudeEvent::MessageStop => {
                let outcome = StreamingOutcome {
                    success: !matches!(self.stop_reason.as_deref(), Some("error")),
                    tokens_used: self.final_usage,
                    final_text: std::mem::take(&mut self.accumulated).into(),
                    tool_calls: Vec::new(),
                    exit_code: Some(0),
                };
                Ok(vec![ZhenfaStreamingEvent::Finished(outcome)])
            }
            ClaudeEvent::Ping => Ok(Vec::new()),
            ClaudeEvent::Error { error } => Ok(vec![ZhenfaStreamingEvent::Error {
                code: error.error_type.into(),
                message: error.message.into(),
            }]),
        }
    }

    fn finalize(&mut self) -> Result<Option<ZhenfaStreamingEvent>, String> {
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
        self.tool_input_buffers.clear();
        self.final_usage = None;
        self.stop_reason = None;
    }

    fn provider_name(&self) -> &'static str {
        "claude"
    }
}
