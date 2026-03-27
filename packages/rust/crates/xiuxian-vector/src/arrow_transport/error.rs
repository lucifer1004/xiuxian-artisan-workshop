use arrow::error::ArrowError;
use reqwest::StatusCode;
use thiserror::Error;

use super::config::ArrowTransportConfigError;

/// Error returned when Arrow transport cannot complete.
#[derive(Debug, Error)]
pub enum ArrowTransportError {
    /// The transport config is invalid.
    #[error("invalid Arrow transport config: {0}")]
    Config(#[from] ArrowTransportConfigError),
    /// The reqwest client could not be built.
    #[error("failed to build Arrow transport HTTP client: {0}")]
    BuildClient(#[source] reqwest::Error),
    /// The HTTP request failed.
    #[error("Arrow transport HTTP request failed: {0}")]
    Http(#[source] reqwest::Error),
    /// The endpoint returned a non-success status code.
    #[error("Arrow transport endpoint returned unexpected status {status}: {body}")]
    UnexpectedStatus {
        /// HTTP status code returned by the remote endpoint.
        status: StatusCode,
        /// Response body captured for diagnostics.
        body: String,
    },
    /// The endpoint returned an unexpected content type.
    #[error("Arrow transport response content-type must start with `{expected}`, found `{found}`")]
    UnexpectedContentType {
        /// Expected content-type prefix.
        expected: String,
        /// Observed content-type or placeholder.
        found: String,
    },
    /// The endpoint returned an incompatible schema version header.
    #[error("Arrow transport response schema version must be `{expected}`, found `{found}`")]
    UnexpectedSchemaVersion {
        /// Expected schema version.
        expected: String,
        /// Observed schema version or placeholder.
        found: String,
    },
    /// The caller supplied no request batches.
    #[error("Arrow transport request batches cannot be empty")]
    EmptyRequest,
    /// Arrow IPC request encoding failed.
    #[error("failed to encode Arrow IPC request: {0}")]
    Encode(#[source] ArrowError),
    /// Arrow IPC response decoding failed.
    #[error("failed to decode Arrow IPC response: {0}")]
    Decode(#[source] ArrowError),
}
