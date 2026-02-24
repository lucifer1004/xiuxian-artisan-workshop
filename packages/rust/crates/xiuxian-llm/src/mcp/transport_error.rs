//! Transport-level error classification helpers.

/// Classified MCP transport error metadata.
#[derive(Debug, Clone, Copy)]
pub struct TransportErrorClass {
    /// Stable error class name for logs/metrics.
    pub kind: &'static str,
    /// Whether reconnect/retry should be attempted.
    pub retryable: bool,
}

/// Classify generic transport failures into stable categories.
#[must_use]
pub fn classify_transport_error(error: &anyhow::Error) -> TransportErrorClass {
    let message = format!("{error:#}").to_lowercase();
    if message.contains("embedding timed out") {
        return TransportErrorClass {
            kind: "tool_embedding_timeout",
            retryable: false,
        };
    }
    if message.contains("transport send error") || message.contains("error sending request") {
        return TransportErrorClass {
            kind: "transport_send",
            retryable: true,
        };
    }
    if message.contains("connection refused") {
        return TransportErrorClass {
            kind: "connection_refused",
            retryable: true,
        };
    }
    if message.contains("connection reset") {
        return TransportErrorClass {
            kind: "connection_reset",
            retryable: true,
        };
    }
    if message.contains("broken pipe") {
        return TransportErrorClass {
            kind: "broken_pipe",
            retryable: true,
        };
    }
    if message.contains("connection closed") || message.contains("channel closed") {
        return TransportErrorClass {
            kind: "channel_closed",
            retryable: true,
        };
    }
    if message.contains("timed out") || message.contains("timeout") {
        return TransportErrorClass {
            kind: "timeout",
            retryable: true,
        };
    }
    if message.contains("client error") {
        return TransportErrorClass {
            kind: "client_error",
            retryable: true,
        };
    }
    if message.contains("dns") || message.contains("name or service not known") {
        return TransportErrorClass {
            kind: "dns_error",
            retryable: true,
        };
    }
    TransportErrorClass {
        kind: "non_transport",
        retryable: false,
    }
}

/// Decide whether an error should trigger reconnect/retry path.
#[must_use]
pub fn should_retry_transport_error(error: &anyhow::Error) -> bool {
    classify_transport_error(error).retryable
}
