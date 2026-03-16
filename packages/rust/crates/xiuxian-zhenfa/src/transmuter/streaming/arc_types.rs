//! High-performance streaming types with Arc<str> optimization.
//!
//! This module provides memory-efficient alternatives to the standard
//! streaming types, using `Arc<str>` for shared text content to minimize
//! allocations during high-throughput streaming scenarios.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::sync::Arc;

/// Performance-optimized streaming event with Arc<str> for shared text.
///
/// This variant reduces memory allocations by using reference-counted
/// strings for text content, enabling zero-copy sharing across multiple
/// consumers without data duplication.
#[derive(Debug, Clone, PartialEq)]
pub enum ArcStreamingEvent {
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
    /// System status message.
    Status(Arc<str>),
    /// Progress indicator with percentage.
    Progress {
        /// Current step description.
        message: Arc<str>,
        /// Progress percentage (0-100).
        percent: u8,
    },
    /// End of stream with final outcome.
    Finished(ArcStreamingOutcome),
    /// Error occurred during streaming.
    Error {
        /// Error code or type.
        code: Arc<str>,
        /// Human-readable error message.
        message: Arc<str>,
    },
}

// Manual Serialize/Deserialize implementation to handle the custom serialization
impl Serialize for ArcStreamingEvent {
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

impl<'de> Deserialize<'de> for ArcStreamingEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ArcStreamingEventVisitor;

        impl<'de> Visitor<'de> for ArcStreamingEventVisitor {
            type Value = ArcStreamingEvent;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an ArcStreamingEvent object")
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
                let mut outcome: Option<ArcStreamingOutcome> = None;

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

                let event_type = event_type.ok_or_else(|| de::Error::missing_field("type"))?;

                match event_type.as_str() {
                    "Thought" => Ok(ArcStreamingEvent::Thought(
                        text.ok_or_else(|| de::Error::missing_field("text"))?,
                    )),
                    "TextDelta" => Ok(ArcStreamingEvent::TextDelta(
                        text.ok_or_else(|| de::Error::missing_field("text"))?,
                    )),
                    "ToolCall" => Ok(ArcStreamingEvent::ToolCall {
                        id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                        name: name.ok_or_else(|| de::Error::missing_field("name"))?,
                        input: input.unwrap_or(Value::Null),
                    }),
                    "ToolResult" => Ok(ArcStreamingEvent::ToolResult {
                        id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                        output: output.unwrap_or(Value::Null),
                    }),
                    "Status" => Ok(ArcStreamingEvent::Status(
                        text.ok_or_else(|| de::Error::missing_field("text"))?,
                    )),
                    "Progress" => Ok(ArcStreamingEvent::Progress {
                        message: message.ok_or_else(|| de::Error::missing_field("message"))?,
                        percent: percent.ok_or_else(|| de::Error::missing_field("percent"))?,
                    }),
                    "Finished" => Ok(ArcStreamingEvent::Finished(
                        outcome.ok_or_else(|| de::Error::missing_field("outcome"))?,
                    )),
                    "Error" => Ok(ArcStreamingEvent::Error {
                        code: code.ok_or_else(|| de::Error::missing_field("code"))?,
                        message: message.ok_or_else(|| de::Error::missing_field("message"))?,
                    }),
                    _ => Err(de::Error::custom(format!(
                        "unknown event type: {}",
                        event_type
                    ))),
                }
            }
        }

        deserializer.deserialize_map(ArcStreamingEventVisitor)
    }
}

impl ArcStreamingEvent {
    /// Create a Thought event from a string.
    #[must_use]
    pub fn thought(text: impl Into<Arc<str>>) -> Self {
        Self::Thought(text.into())
    }

    /// Create a TextDelta event from a string.
    #[must_use]
    pub fn text_delta(text: impl Into<Arc<str>>) -> Self {
        Self::TextDelta(text.into())
    }

    /// Create a Status event from a string.
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

/// Performance-optimized streaming outcome with Arc<str>.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcStreamingOutcome {
    /// Whether the stream completed successfully.
    pub success: bool,
    /// Token usage statistics.
    pub tokens_used: Option<ArcTokenUsage>,
    /// Final accumulated text.
    pub final_text: Arc<str>,
    /// Tool calls made during the stream.
    pub tool_calls: Vec<ArcToolCallRecord>,
    /// Exit code if available.
    pub exit_code: Option<i32>,
}

impl Serialize for ArcStreamingOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ArcStreamingOutcome", 5)?;
        s.serialize_field("success", &self.success)?;
        s.serialize_field("tokens_used", &self.tokens_used)?;
        s.serialize_field("final_text", self.final_text.as_ref())?;
        s.serialize_field("tool_calls", &self.tool_calls)?;
        s.serialize_field("exit_code", &self.exit_code)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ArcStreamingOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct ArcStreamingOutcomeVisitor;

        impl<'de> Visitor<'de> for ArcStreamingOutcomeVisitor {
            type Value = ArcStreamingOutcome;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an ArcStreamingOutcome object")
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

                Ok(ArcStreamingOutcome {
                    success: success.unwrap_or(false),
                    tokens_used,
                    final_text: final_text.unwrap_or_else(|| Arc::from("")),
                    tool_calls: tool_calls.unwrap_or_default(),
                    exit_code,
                })
            }
        }

        deserializer.deserialize_map(ArcStreamingOutcomeVisitor)
    }
}

impl ArcStreamingOutcome {
    /// Create a successful outcome.
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

    /// Create a failure outcome.
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

/// Token usage with Arc optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcTokenUsage {
    /// Input tokens consumed.
    pub input: u64,
    /// Output tokens produced.
    pub output: u64,
    /// Total tokens (input + output).
    pub total: u64,
}

impl ArcTokenUsage {
    /// Create new token usage.
    #[must_use]
    pub const fn new(input: u64, output: u64) -> Self {
        Self {
            input,
            output,
            total: input + output,
        }
    }
}

/// Tool call record with Arc<str> optimization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcToolCallRecord {
    /// Tool call identifier.
    pub id: Arc<str>,
    /// Tool name.
    pub name: Arc<str>,
    /// Whether the tool call succeeded.
    pub succeeded: bool,
}

impl Serialize for ArcToolCallRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ArcToolCallRecord", 3)?;
        s.serialize_field("id", self.id.as_ref())?;
        s.serialize_field("name", self.name.as_ref())?;
        s.serialize_field("succeeded", &self.succeeded)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ArcToolCallRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ArcToolCallRecordVisitor;

        impl<'de> Visitor<'de> for ArcToolCallRecordVisitor {
            type Value = ArcToolCallRecord;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an ArcToolCallRecord object")
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

                Ok(ArcToolCallRecord {
                    id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                    name: name.ok_or_else(|| de::Error::missing_field("name"))?,
                    succeeded: succeeded.unwrap_or(false),
                })
            }
        }

        deserializer.deserialize_map(ArcToolCallRecordVisitor)
    }
}

impl ArcToolCallRecord {
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

/// Event buffer for batched processing with minimal allocations.
#[derive(Debug, Default)]
pub struct EventBuffer {
    /// Buffered events.
    events: Vec<ArcStreamingEvent>,
    /// Total estimated size.
    total_size: usize,
    /// Maximum buffer size before flush.
    max_size: usize,
}

impl EventBuffer {
    /// Create a new event buffer with default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Create a new event buffer with specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            total_size: 0,
            max_size: 1024 * 1024, // 1MB default max
        }
    }

    /// Set the maximum buffer size before forced flush.
    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;
    }

    /// Push an event into the buffer.
    pub fn push(&mut self, event: ArcStreamingEvent) {
        self.total_size += event.estimated_size();
        self.events.push(event);
    }

    /// Check if the buffer should be flushed.
    #[must_use]
    pub fn should_flush(&self) -> bool {
        self.total_size >= self.max_size || self.events.len() >= self.events.capacity()
    }

    /// Get the current number of events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Drain all events from the buffer.
    pub fn drain(&mut self) -> impl Iterator<Item = ArcStreamingEvent> + '_ {
        self.total_size = 0;
        self.events.drain(..)
    }

    /// Clear the buffer without consuming events.
    pub fn clear(&mut self) {
        self.events.clear();
        self.total_size = 0;
    }

    /// Get the total estimated size.
    #[must_use]
    pub fn total_size(&self) -> usize {
        self.total_size
    }
}

/// Conversion from standard streaming event to Arc-optimized event.
impl From<super::ZhenfaStreamingEvent> for ArcStreamingEvent {
    fn from(event: super::ZhenfaStreamingEvent) -> Self {
        match event {
            super::ZhenfaStreamingEvent::Thought(text) => Self::Thought(text.into()),
            super::ZhenfaStreamingEvent::TextDelta(text) => Self::TextDelta(text.into()),
            super::ZhenfaStreamingEvent::ToolCall { id, name, input } => Self::ToolCall {
                id: id.into(),
                name: name.into(),
                input,
            },
            super::ZhenfaStreamingEvent::ToolResult { id, output } => Self::ToolResult {
                id: id.into(),
                output,
            },
            super::ZhenfaStreamingEvent::Status(text) => Self::Status(text.into()),
            super::ZhenfaStreamingEvent::Progress { message, percent } => Self::Progress {
                message: message.into(),
                percent,
            },
            super::ZhenfaStreamingEvent::Finished(outcome) => Self::Finished(ArcStreamingOutcome {
                success: outcome.success,
                tokens_used: outcome
                    .tokens_used
                    .map(|t| ArcTokenUsage::new(t.input, t.output)),
                final_text: outcome.final_text.into(),
                tool_calls: outcome
                    .tool_calls
                    .into_iter()
                    .map(|tc| ArcToolCallRecord::new(tc.id, tc.name, tc.succeeded))
                    .collect(),
                exit_code: outcome.exit_code,
            }),
            super::ZhenfaStreamingEvent::Error { code, message } => Self::Error {
                code: code.into(),
                message: message.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_event_creates_thought() {
        let event = ArcStreamingEvent::thought("Test thought");
        assert!(matches!(event, ArcStreamingEvent::Thought(_)));
        assert_eq!(event.text_content(), Some("Test thought"));
    }

    #[test]
    fn arc_event_estimates_size() {
        let event = ArcStreamingEvent::thought("Hello world");
        assert!(event.estimated_size() > 11);
    }

    #[test]
    fn event_buffer_pushes_and_drains() {
        let mut buffer = EventBuffer::with_capacity(10);

        buffer.push(ArcStreamingEvent::thought("Event 1"));
        buffer.push(ArcStreamingEvent::thought("Event 2"));

        assert_eq!(buffer.len(), 2);
        assert!(!buffer.is_empty());

        let drained: Vec<_> = buffer.drain().collect();
        assert_eq!(drained.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn event_buffer_flush_threshold() {
        let mut buffer = EventBuffer::new();
        // Set a very low threshold to trigger flush on size
        buffer.set_max_size(100);

        // Add small event
        buffer.push(ArcStreamingEvent::thought("Hi"));

        // Add large event that should exceed threshold
        buffer.push(ArcStreamingEvent::thought(
            "This is a longer text that should push us over the threshold",
        ));

        // Total size should exceed 100 bytes now
        assert!(
            buffer.total_size() >= 100,
            "Total size should be at least 100"
        );
        assert!(
            buffer.should_flush(),
            "Buffer should flush when size threshold exceeded"
        );
    }

    #[test]
    fn converts_from_standard_event() {
        let standard = super::super::ZhenfaStreamingEvent::Thought(std::sync::Arc::from("test"));
        let arc: ArcStreamingEvent = standard.into();

        assert_eq!(arc.text_content(), Some("test"));
    }
}
