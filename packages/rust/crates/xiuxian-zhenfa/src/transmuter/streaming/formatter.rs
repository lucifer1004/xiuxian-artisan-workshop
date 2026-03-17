//! Output formatting utilities for streaming events.

use super::ZhenfaStreamingEvent;

/// Display style for formatted output.
#[derive(Debug, Clone, Copy, Default)]
pub enum DisplayStyle {
    /// Plain text output.
    #[default]
    Plain,
    /// ANSI-colored terminal output.
    Ansi,
    /// JSON format.
    Json,
}

/// ANSI formatter for streaming events.
#[derive(Debug, Default)]
pub struct AnsiFormatter {
    style: DisplayStyle,
}

impl AnsiFormatter {
    /// Create a new ANSI formatter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a formatter with a specific style.
    #[must_use]
    pub fn with_style(style: DisplayStyle) -> Self {
        Self { style }
    }

    /// Format a streaming event for display.
    #[must_use]
    pub fn format(&self, event: &ZhenfaStreamingEvent) -> String {
        match self.style {
            DisplayStyle::Plain => format!("{event:?}"),
            DisplayStyle::Ansi => format!("\x1b[36m{event:?}\x1b[0m"),
            DisplayStyle::Json => {
                serde_json::to_string(event).unwrap_or_else(|_| "null".to_string())
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/transmuter/streaming/formatter.rs"]
mod tests;
