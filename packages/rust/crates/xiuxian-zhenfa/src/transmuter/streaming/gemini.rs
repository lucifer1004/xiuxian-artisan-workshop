//! Gemini CLI streaming parser.
//!
//! Parses event-stream output from Gemini CLI, mapping native events
//! to the unified `ZhenfaStreamingEvent` model.

use super::ZhenfaStreamingEvent;
use super::traits::{StreamingOutcome, StreamingTransmuter, TokenUsage};
use serde::Deserialize;

/// Gemini streaming response structure.
#[derive(Debug, Clone, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
struct Candidate {
    content: Content,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Content {
    parts: Vec<Part>,
    #[serde(default)]
    role: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Part {
    Text {
        text: String,
    },
    Thought {
        thought: String,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        function_call: FunctionCall,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct FunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageMetadata {
    prompt_token_count: u64,
    candidates_token_count: u64,
    total_token_count: u64,
}

/// Parser for Gemini CLI streaming output.
#[derive(Debug, Default)]
pub struct GeminiStreamingParser {
    accumulated: String,
    final_usage: Option<TokenUsage>,
    finish_reason: Option<String>,
}

impl GeminiStreamingParser {
    /// Create a new Gemini streaming parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StreamingTransmuter for GeminiStreamingParser {
    fn parse_line(&mut self, line: &str) -> Result<Vec<ZhenfaStreamingEvent>, String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(Vec::new());
        }

        // Handle SSE format
        let json_str = if let Some(stripped) = line.strip_prefix("data: ") {
            stripped
        } else if line.starts_with(':') {
            // Comment line, ignore
            return Ok(Vec::new());
        } else {
            line
        };

        // Check for stream end
        if json_str == "[DONE]" || json_str == "[END]" {
            let outcome = StreamingOutcome {
                success: true,
                tokens_used: self.final_usage,
                final_text: std::mem::take(&mut self.accumulated).into(),
                tool_calls: Vec::new(),
                exit_code: Some(0),
            };
            return Ok(vec![ZhenfaStreamingEvent::Finished(outcome)]);
        }

        let response: GeminiResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse Gemini response: {e}"))?;

        // Extract usage if present
        if let Some(usage) = response.usage_metadata {
            self.final_usage = Some(TokenUsage {
                input: usage.prompt_token_count,
                output: usage.candidates_token_count,
                total: usage.total_token_count,
            });
        }

        // Process candidates - return ALL events from the chunk
        let mut events = Vec::new();
        for candidate in &response.candidates {
            if let Some(reason) = &candidate.finish_reason {
                self.finish_reason = Some(reason.clone());
            }

            for part in &candidate.content.parts {
                match part {
                    Part::Text { text } => {
                        self.accumulated.push_str(text);
                        events.push(ZhenfaStreamingEvent::TextDelta(text.clone().into()));
                    }
                    Part::Thought { thought } => {
                        events.push(ZhenfaStreamingEvent::Thought(thought.clone().into()));
                    }
                    Part::FunctionCall { function_call } => {
                        events.push(ZhenfaStreamingEvent::ToolCall {
                            id: format!("gemini_{}", function_call.name).into(),
                            name: function_call.name.clone().into(),
                            input: function_call.args.clone(),
                        });
                    }
                }
            }
        }

        // Return ALL events (fixes critical bug where only first event was returned)
        Ok(events)
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
        self.final_usage = None;
        self.finish_reason = None;
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }
}
