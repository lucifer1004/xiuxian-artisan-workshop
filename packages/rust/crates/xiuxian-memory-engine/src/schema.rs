//! Schema definitions for xiuxian-memory-engine data contracts.
//!
//! Follows project Schema Singularity: Rust as SSOT, strict validation, fail fast.
//! See `docs/reference/schema-singularity.md` and `tool_record_validation.py`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Episode;

/// Error when episode metadata fails schema validation.
#[derive(Debug, Error)]
pub enum EpisodeMetadataError {
    /// JSON deserialization failed.
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// Required field missing or invalid type.
    #[error("Schema validation: {0}")]
    Validation(String),

    /// Q-value out of valid range [0.0, 1.0].
    #[error("q_value must be in [0.0, 1.0], got {0}")]
    InvalidQValue(f32),

    /// Count fields must be non-negative.
    #[error("retrieval_count, success_count, and failure_count must be >= 0")]
    InvalidCount,
}

/// Episode metadata stored in `LanceDB`.
///
/// Contract: All fields required for strict validation. Fail fast on invalid data.
/// Used when serializing/deserializing episode metadata in vector store.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EpisodeMetadata {
    /// The actual experience (response/action taken).
    pub experience: String,
    /// The outcome (success indicator, error message, etc.).
    pub outcome: String,
    /// Current Q-value (learned utility). Must be in [0.0, 1.0].
    #[schemars(range(min = 0.0, max = 1.0))]
    pub q_value: f32,
    /// Number of retrieval or access events.
    pub retrieval_count: u32,
    /// Number of successful retrievals.
    pub success_count: u32,
    /// Number of failed retrievals.
    pub failure_count: u32,
    /// Creation timestamp (Unix milliseconds).
    pub created_at: i64,
    /// Last update timestamp (Unix milliseconds).
    pub updated_at: i64,
}

impl EpisodeMetadata {
    /// Validate and create from an [`Episode`].
    ///
    /// # Errors
    ///
    /// Returns an error if `q_value` is outside `[0.0, 1.0]`.
    pub fn from_episode(episode: &Episode) -> Result<Self, EpisodeMetadataError> {
        if !(0.0..=1.0).contains(&episode.q_value) {
            return Err(EpisodeMetadataError::InvalidQValue(episode.q_value));
        }
        Ok(Self {
            experience: episode.experience.clone(),
            outcome: episode.outcome.clone(),
            q_value: episode.q_value,
            retrieval_count: episode.retrieval_count,
            success_count: episode.success_count,
            failure_count: episode.failure_count,
            created_at: episode.created_at,
            updated_at: episode.updated_at,
        })
    }

    /// Deserialize from JSON string with strict validation.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON is invalid, empty, or contains invalid `q_value`.
    pub fn from_json(s: &str) -> Result<Self, EpisodeMetadataError> {
        if s.trim().is_empty() {
            return Err(EpisodeMetadataError::Validation(
                "Empty metadata string".to_string(),
            ));
        }
        let meta: Self = serde_json::from_str(s)?;
        if !(0.0..=1.0).contains(&meta.q_value) {
            return Err(EpisodeMetadataError::InvalidQValue(meta.q_value));
        }
        Ok(meta)
    }

    /// Serialize to JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, EpisodeMetadataError> {
        serde_json::to_string(self).map_err(Into::into)
    }
}

impl Default for EpisodeMetadata {
    fn default() -> Self {
        Self {
            experience: String::new(),
            outcome: String::new(),
            q_value: 0.5,
            retrieval_count: 0,
            success_count: 0,
            failure_count: 0,
            created_at: 0,
            updated_at: 0,
        }
    }
}
