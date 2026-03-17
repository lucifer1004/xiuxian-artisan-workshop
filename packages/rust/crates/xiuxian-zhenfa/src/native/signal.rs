//! Runtime signal payloads emitted by native zhenfa tools.

use serde::{Deserialize, Serialize};

/// Asynchronous fire-and-forget signal emitted during native tool execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZhenfaSignal {
    /// Reinforcement-learning reward signal.
    Reward {
        /// Episode identifier to update.
        episode_id: String,
        /// Reward value, typically in `[0.0, 1.0]`.
        value: f32,
        /// Signal source identifier for audit correlation.
        source: String,
    },
    /// Execution trace signal for observability and diagnostics.
    Trace {
        /// Node or component identifier.
        node_id: String,
        /// Trace event payload.
        event: String,
    },
    /// Semantic drift signal for documentation synchronization.
    ///
    /// Emitted when source code changes may affect documentation with `:OBSERVE:` patterns.
    SemanticDrift {
        /// The source file that changed.
        source_path: String,
        /// File stem used for heuristic matching.
        file_stem: String,
        /// Number of affected documents.
        affected_count: usize,
        /// Confidence level: "high", "medium", or "low".
        confidence: String,
        /// Human-readable summary.
        summary: String,
    },
}
