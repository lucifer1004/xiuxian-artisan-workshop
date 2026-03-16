//! Unified streaming parser for multi-agent CLI outputs.
//!
//! This module provides a common abstraction for parsing streaming output
//! from various LLM CLI tools (Claude Code, Gemini CLI, Codex) into a unified
//! event stream that can be consumed by Qianji nodes.
//!
//! # Zero-Copy Architecture
//!
//! All text content uses `Arc<str>` for zero-copy sharing across consumers,
//! eliminating heap allocations for each text delta in high-throughput scenarios.

mod arc_types;
mod claude;
mod codex;
mod formatter;
mod gemini;
mod logic_gate;
mod supervisor;
mod traits;

pub use arc_types::{ArcStreamingOutcome, ArcTokenUsage, ArcToolCallRecord, EventBuffer};
pub use claude::ClaudeStreamingParser;
pub use codex::CodexStreamingParser;
pub use formatter::{AnsiFormatter, DisplayStyle};
pub use gemini::GeminiStreamingParser;
pub use logic_gate::{LogicGate, LogicGateError, LogicGateEvent, XsdConstraintMap};
pub use supervisor::{
    CognitiveDimension, CognitiveEvent, CognitiveSupervisor, SupervisorContext, ThoughtSubcategory,
};
pub use traits::{StreamingOutcome, StreamingTransmuter};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::sync::Arc;

/// Unified event model for all CLI streaming outputs with zero-copy text.
///
/// Every provider must map its native JSON events into this common enum,
/// allowing Qianji nodes to react to intermediate states before the full
/// response is finished.
///
/// # Zero-Copy Guarantee
///
/// All text fields use `Arc<str>` for efficient sharing without heap duplication.
#[derive(Debug, Clone, PartialEq)]
pub enum ZhenfaStreamingEvent {
    /// Chain of thought / reasoning content.
    Thought(Arc<str>),
    /// Direct text output delta.
    TextDelta(Arc<str>),
    /// Agent is requesting a tool invocation.
    ToolCall {
        /// Unique identifier for this tool call.
        id: Arc<str>,
        /// Name of the tool being invoked.
        name: Arc<str>,
        /// Input parameters for the tool.
        input: Value,
    },
    /// Tool execution result.
    ToolResult {
        /// ID of the corresponding tool call.
        id: Arc<str>,
        /// Output from the tool execution.
        output: Value,
    },
    /// System status message (e.g., "Scanning files...").
    Status(Arc<str>),
    /// Progress indicator with percentage (0-100).
    Progress {
        /// Current step description.
        message: Arc<str>,
        /// Progress percentage (0-100).
        percent: u8,
    },
    /// End of stream with final outcome.
    Finished(StreamingOutcome),
    /// Error occurred during streaming.
    Error {
        /// Error code or type.
        code: Arc<str>,
        /// Human-readable error message.
        message: Arc<str>,
    },
}

// Manual Serialize implementation for Arc<str> fields
impl Serialize for ZhenfaStreamingEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        match self {
            Self::Thought(text) => {
                map.serialize_entry("type", "Thought")?;
                map.serialize_entry("text", text.as_ref())?;
            }
            Self::TextDelta(text) => {
                map.serialize_entry("type", "TextDelta")?;
                map.serialize_entry("text", text.as_ref())?;
            }
            Self::ToolCall { id, name, input } => {
                map.serialize_entry("type", "ToolCall")?;
                map.serialize_entry("id", id.as_ref())?;
                map.serialize_entry("name", name.as_ref())?;
                map.serialize_entry("input", input)?;
            }
            Self::ToolResult { id, output } => {
                map.serialize_entry("type", "ToolResult")?;
                map.serialize_entry("id", id.as_ref())?;
                map.serialize_entry("output", output)?;
            }
            Self::Status(text) => {
                map.serialize_entry("type", "Status")?;
                map.serialize_entry("text", text.as_ref())?;
            }
            Self::Progress { message, percent } => {
                map.serialize_entry("type", "Progress")?;
                map.serialize_entry("message", message.as_ref())?;
                map.serialize_entry("percent", percent)?;
            }
            Self::Finished(outcome) => {
                map.serialize_entry("type", "Finished")?;
                map.serialize_entry("outcome", outcome)?;
            }
            Self::Error { code, message } => {
                map.serialize_entry("type", "Error")?;
                map.serialize_entry("code", code.as_ref())?;
                map.serialize_entry("message", message.as_ref())?;
            }
        }
        map.end()
    }
}

// Manual Deserialize implementation for Arc<str> fields
impl<'de> Deserialize<'de> for ZhenfaStreamingEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct ZhenfaStreamingEventVisitor;

        impl<'de> Visitor<'de> for ZhenfaStreamingEventVisitor {
            type Value = ZhenfaStreamingEvent;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a ZhenfaStreamingEvent object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut event_type: Option<String> = None;
                let mut text: Option<Arc<str>> = None;
                let mut id: Option<Arc<str>> = None;
                let mut name: Option<Arc<str>> = None;
                let mut input: Option<Value> = None;
                let mut output: Option<Value> = None;
                let mut percent: Option<u8> = None;
                let mut message: Option<Arc<str>> = None;
                let mut code: Option<Arc<str>> = None;
                let mut outcome: Option<StreamingOutcome> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => event_type = Some(map.next_value()?),
                        "text" => text = Some(Arc::from(map.next_value::<String>()?)),
                        "id" => id = Some(Arc::from(map.next_value::<String>()?)),
                        "name" => name = Some(Arc::from(map.next_value::<String>()?)),
                        "input" => input = Some(map.next_value()?),
                        "output" => output = Some(map.next_value()?),
                        "percent" => percent = Some(map.next_value()?),
                        "message" => message = Some(Arc::from(map.next_value::<String>()?)),
                        "code" => code = Some(Arc::from(map.next_value::<String>()?)),
                        "outcome" => outcome = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>();
                        }
                    }
                }

                let event_type =
                    event_type.ok_or_else(|| serde::de::Error::missing_field("type"))?;

                match event_type.as_str() {
                    "Thought" => Ok(ZhenfaStreamingEvent::Thought(
                        text.ok_or_else(|| serde::de::Error::missing_field("text"))?,
                    )),
                    "TextDelta" => Ok(ZhenfaStreamingEvent::TextDelta(
                        text.ok_or_else(|| serde::de::Error::missing_field("text"))?,
                    )),
                    "ToolCall" => Ok(ZhenfaStreamingEvent::ToolCall {
                        id: id.ok_or_else(|| serde::de::Error::missing_field("id"))?,
                        name: name.ok_or_else(|| serde::de::Error::missing_field("name"))?,
                        input: input.unwrap_or(Value::Null),
                    }),
                    "ToolResult" => Ok(ZhenfaStreamingEvent::ToolResult {
                        id: id.ok_or_else(|| serde::de::Error::missing_field("id"))?,
                        output: output.unwrap_or(Value::Null),
                    }),
                    "Status" => Ok(ZhenfaStreamingEvent::Status(
                        text.ok_or_else(|| serde::de::Error::missing_field("text"))?,
                    )),
                    "Progress" => Ok(ZhenfaStreamingEvent::Progress {
                        message: message
                            .ok_or_else(|| serde::de::Error::missing_field("message"))?,
                        percent: percent
                            .ok_or_else(|| serde::de::Error::missing_field("percent"))?,
                    }),
                    "Finished" => Ok(ZhenfaStreamingEvent::Finished(
                        outcome.ok_or_else(|| serde::de::Error::missing_field("outcome"))?,
                    )),
                    "Error" => Ok(ZhenfaStreamingEvent::Error {
                        code: code.ok_or_else(|| serde::de::Error::missing_field("code"))?,
                        message: message
                            .ok_or_else(|| serde::de::Error::missing_field("message"))?,
                    }),
                    _ => Err(serde::de::Error::custom(format!(
                        "unknown event type: {}",
                        event_type
                    ))),
                }
            }
        }

        deserializer.deserialize_map(ZhenfaStreamingEventVisitor)
    }
}

impl ZhenfaStreamingEvent {
    /// Create a Thought event.
    #[must_use]
    pub fn thought(text: impl Into<Arc<str>>) -> Self {
        Self::Thought(text.into())
    }

    /// Create a TextDelta event.
    #[must_use]
    pub fn text_delta(text: impl Into<Arc<str>>) -> Self {
        Self::TextDelta(text.into())
    }

    /// Create a Status event.
    #[must_use]
    pub fn status(text: impl Into<Arc<str>>) -> Self {
        Self::Status(text.into())
    }

    /// Check if this event represents terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Finished(_) | Self::Error { .. })
    }

    /// Check if this event contains tool-related content.
    #[must_use]
    pub const fn is_tool_event(&self) -> bool {
        matches!(self, Self::ToolCall { .. } | Self::ToolResult { .. })
    }

    /// Extract text content if this is a text or thought event.
    #[must_use]
    pub fn text_content(&self) -> Option<&str> {
        match self {
            Self::Thought(text) | Self::TextDelta(text) | Self::Status(text) => Some(text),
            _ => None,
        }
    }

    /// Get the estimated memory size in bytes.
    #[must_use]
    pub fn estimated_size(&self) -> usize {
        match self {
            Self::Thought(text) | Self::TextDelta(text) | Self::Status(text) => {
                text.len() + std::mem::size_of::<Self>()
            }
            Self::ToolCall { id, name, input } => {
                id.len() + name.len() + input.to_string().len() + std::mem::size_of::<Self>()
            }
            Self::ToolResult { id, output } => {
                id.len() + output.to_string().len() + std::mem::size_of::<Self>()
            }
            Self::Progress { message, .. } => message.len() + std::mem::size_of::<Self>(),
            Self::Finished(outcome) => outcome.estimated_size() + std::mem::size_of::<Self>(),
            Self::Error { code, message } => {
                code.len() + message.len() + std::mem::size_of::<Self>()
            }
        }
    }
}

/// Provider identifier for streaming parsers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamingProvider {
    /// Claude Code CLI (Anthropic).
    Claude,
    /// Gemini CLI (Google).
    Gemini,
    /// Codex / OpenAI-style agents.
    Codex,
}

impl StreamingProvider {
    /// Get the appropriate parser for this provider.
    #[must_use]
    pub fn parser(&self) -> Box<dyn StreamingTransmuter> {
        match self {
            Self::Claude => Box::new(ClaudeStreamingParser::new()),
            Self::Gemini => Box::new(GeminiStreamingParser::new()),
            Self::Codex => Box::new(CodexStreamingParser::new()),
        }
    }
}

impl std::fmt::Display for StreamingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Gemini => write!(f, "gemini"),
            Self::Codex => write!(f, "codex"),
        }
    }
}

/// Detect the streaming provider from the first line of output.
///
/// # Errors
///
/// Returns an error string if the provider cannot be determined.
pub fn detect_provider(first_line: &str) -> Result<StreamingProvider, String> {
    let trimmed = first_line.trim();

    // Claude uses NDJSON with specific event types
    if trimmed.starts_with("{\"type\":\"") && trimmed.contains("\"message\"") {
        return Ok(StreamingProvider::Claude);
    }

    // Gemini uses event-stream format
    if trimmed.starts_with("data:") || trimmed.contains("\"candidates\"") {
        return Ok(StreamingProvider::Gemini);
    }

    // Codex/OpenAI style
    if trimmed.contains("\"choices\"") || trimmed.contains("\"delta\"") {
        return Ok(StreamingProvider::Codex);
    }

    Err(format!(
        "Unable to detect streaming provider from: {}",
        &trimmed[..trimmed.len().min(100)]
    ))
}
