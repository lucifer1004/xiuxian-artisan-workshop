//! Core traits for streaming transmuters.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

use super::ZhenfaStreamingEvent;

/// Final outcome of a streaming session with zero-copy text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingOutcome {
    /// Whether the overall operation succeeded.
    pub success: bool,
    /// Total token usage if available.
    pub tokens_used: Option<TokenUsage>,
    /// Final accumulated text content (zero-copy).
    pub final_text: Arc<str>,
    /// List of tool calls made during the session.
    pub tool_calls: Vec<ToolCallRecord>,
    /// Exit code from the CLI process (if applicable).
    pub exit_code: Option<i32>,
}

impl Serialize for StreamingOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("StreamingOutcome", 5)?;
        s.serialize_field("success", &self.success)?;
        s.serialize_field("tokens_used", &self.tokens_used)?;
        s.serialize_field("final_text", self.final_text.as_ref())?;
        s.serialize_field("tool_calls", &self.tool_calls)?;
        s.serialize_field("exit_code", &self.exit_code)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for StreamingOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct StreamingOutcomeVisitor;

        impl<'de> Visitor<'de> for StreamingOutcomeVisitor {
            type Value = StreamingOutcome;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a StreamingOutcome object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut success = None;
                let mut tokens_used = None;
                let mut final_text = None;
                let mut tool_calls = None;
                let mut exit_code = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "success" => success = Some(map.next_value()?),
                        "tokens_used" => tokens_used = Some(map.next_value()?),
                        "final_text" => final_text = Some(Arc::from(map.next_value::<String>()?)),
                        "tool_calls" => tool_calls = Some(map.next_value()?),
                        "exit_code" => exit_code = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>();
                        }
                    }
                }

                Ok(StreamingOutcome {
                    success: success.unwrap_or(false),
                    tokens_used,
                    final_text: final_text.unwrap_or_else(|| Arc::from("")),
                    tool_calls: tool_calls.unwrap_or_default(),
                    exit_code,
                })
            }
        }

        deserializer.deserialize_map(StreamingOutcomeVisitor)
    }
}

impl StreamingOutcome {
    /// Create a successful outcome with the given text.
    #[must_use]
    pub fn success(text: impl Into<Arc<str>>) -> Self {
        Self {
            success: true,
            tokens_used: None,
            final_text: text.into(),
            tool_calls: Vec::new(),
            exit_code: Some(0),
        }
    }

    /// Create a failed outcome with an error message.
    #[must_use]
    pub fn failure(message: impl Into<Arc<str>>) -> Self {
        Self {
            success: false,
            tokens_used: None,
            final_text: message.into(),
            tool_calls: Vec::new(),
            exit_code: Some(1),
        }
    }

    /// Get the estimated memory size in bytes.
    #[must_use]
    pub fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.final_text.len()
            + self
                .tool_calls
                .iter()
                .map(|tc| tc.estimated_size())
                .sum::<usize>()
    }
}

/// Token usage statistics from a streaming session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens consumed.
    pub input: u64,
    /// Output tokens generated.
    pub output: u64,
    /// Total tokens (may include cached or other).
    pub total: u64,
}

impl TokenUsage {
    /// Create a new token usage record.
    #[must_use]
    pub const fn new(input: u64, output: u64) -> Self {
        Self {
            input,
            output,
            total: input + output,
        }
    }
}

/// Record of a tool call made during streaming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallRecord {
    /// Unique identifier for the tool call (zero-copy).
    pub id: Arc<str>,
    /// Name of the tool invoked (zero-copy).
    pub name: Arc<str>,
    /// Whether the tool call completed successfully.
    pub succeeded: bool,
}

impl Serialize for ToolCallRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ToolCallRecord", 3)?;
        s.serialize_field("id", self.id.as_ref())?;
        s.serialize_field("name", self.name.as_ref())?;
        s.serialize_field("succeeded", &self.succeeded)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ToolCallRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct ToolCallRecordVisitor;

        impl<'de> Visitor<'de> for ToolCallRecordVisitor {
            type Value = ToolCallRecord;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a ToolCallRecord object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut id = None;
                let mut name = None;
                let mut succeeded = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "id" => id = Some(Arc::from(map.next_value::<String>()?)),
                        "name" => name = Some(Arc::from(map.next_value::<String>()?)),
                        "succeeded" => succeeded = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>();
                        }
                    }
                }

                Ok(ToolCallRecord {
                    id: id.ok_or_else(|| serde::de::Error::missing_field("id"))?,
                    name: name.ok_or_else(|| serde::de::Error::missing_field("name"))?,
                    succeeded: succeeded.unwrap_or(false),
                })
            }
        }

        deserializer.deserialize_map(ToolCallRecordVisitor)
    }
}

impl ToolCallRecord {
    /// Create a new tool call record.
    #[must_use]
    pub fn new(id: impl Into<Arc<str>>, name: impl Into<Arc<str>>, succeeded: bool) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            succeeded,
        }
    }

    /// Get the estimated memory size in bytes.
    #[must_use]
    pub fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.id.len() + self.name.len()
    }
}

/// Trait for streaming CLI output parsers.
///
/// Each provider (Claude, Gemini, Codex) implements this trait to convert
/// their native streaming format into `ZhenfaStreamingEvent`s.
pub trait StreamingTransmuter: Send + Sync {
    /// Parse a single line of streaming output.
    ///
    /// Returns a vector of events parsed from the line. May be empty if
    /// more data is needed, or contain multiple events for providers like
    /// Gemini that batch thoughts and text in single chunks.
    ///
    /// # Errors
    ///
    /// Returns an error string if the line cannot be parsed.
    fn parse_line(&mut self, line: &str) -> Result<Vec<ZhenfaStreamingEvent>, String>;

    /// Process accumulated buffer and emit any pending events.
    ///
    /// Should be called when the stream ends to flush any remaining state.
    ///
    /// # Errors
    ///
    /// Returns an error string if finalization fails.
    fn finalize(&mut self) -> Result<Option<ZhenfaStreamingEvent>, String>;

    /// Get the current accumulated text content.
    fn accumulated_text(&self) -> &str;

    /// Reset the parser state for a new streaming session.
    fn reset(&mut self);

    /// Get the provider name for logging purposes.
    fn provider_name(&self) -> &'static str;
}

/// Blanket implementation for common streaming behavior.
pub(crate) fn accumulate_text(buffer: &mut String, delta: &str) {
    if !delta.is_empty() {
        buffer.push_str(delta);
    }
}

/// Helper to strip NDJSON prefix if present.
pub(crate) fn strip_ndjson_prefix(line: &str) -> &str {
    line.strip_prefix("data: ").unwrap_or(line).trim()
}

/// Helper to check if a line is a keep-alive or comment.
pub(crate) fn is_ignorable_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed == ":" || trimmed.starts_with("//")
}
