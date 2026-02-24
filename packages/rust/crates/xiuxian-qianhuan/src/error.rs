use std::fmt::{Display, Formatter};

/// Parse and validation errors for prompt injection payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum InjectionError {
    /// Input XML payload is empty after trimming.
    EmptyPayload,
    /// Payload does not contain any parseable `<qa>` blocks.
    MissingQaBlock,
    /// A `<qa>` block is missing `<q>`.
    MissingQuestion,
    /// A `<qa>` block is missing `<a>`.
    MissingAnswer,
    /// Detected potential prompt injection or context drift.
    ContextDrift(String),
    /// XML structure validation failed (unbalanced tags or illegal nesting).
    XmlValidationError(String),
    /// Context is insufficient to ground the persona (CCS too low).
    /// Carries a description of what is missing.
    ContextInsufficient {
        /// Context Confidence Score for grounding quality.
        ccs: f64,
        /// Human-readable explanation of missing grounding context.
        missing_info: String,
    },
}

impl Display for InjectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPayload => write!(f, "injection payload is empty"),
            Self::MissingQaBlock => write!(f, "injection payload must contain at least one <qa>"),
            Self::MissingQuestion => write!(f, "<qa> block missing required <q>"),
            Self::MissingAnswer => write!(f, "<qa> block missing required <a>"),
            Self::ContextDrift(msg) => write!(f, "context drift: {msg}"),
            Self::XmlValidationError(msg) => write!(f, "XML validation: {msg}"),
            Self::ContextInsufficient { ccs, missing_info } => {
                write!(
                    f,
                    "insufficient context (CCS: {ccs:.2}). Missing: {missing_info}"
                )
            }
        }
    }
}

impl std::error::Error for InjectionError {}
