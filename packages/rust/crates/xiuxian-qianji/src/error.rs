//! Unified error handling for the Qianji Engine.

use thiserror::Error;

/// Error types emitted during graph compilation or execution.
#[derive(Error, Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum QianjiError {
    /// Failure during graph topology verification or compilation.
    #[error("Graph topology error: {0}")]
    TopologyError(String),

    /// Failure during node execution.
    #[error("Node execution failed: {0}")]
    ExecutionError(String),

    /// Strategic deviation detected during auditing.
    #[error("Strategic drift detected: {0}")]
    DriftError(String),

    /// Internal capacity or resource limit reached.
    #[error("Resource exhaustion: {0}")]
    CapacityError(String),
}
