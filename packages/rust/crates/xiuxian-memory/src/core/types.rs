//! Core types for MemRL state-action space.

use serde::{Deserialize, Serialize};

/// Discrete representation of the Agent's environment state.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryState {
    /// Level of information missingness (0-5, derived from CCS).
    pub context_entropy: u8,
    /// Active persona identifier hash.
    pub persona_hash: u64,
    /// Type of task being performed.
    pub task_kind: String,
}

/// Actions the memory system can take on a specific memory segment.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum MemoryAction {
    /// Keep the memory in episodic storage.
    Retain,
    /// Completely remove the memory to save context budget.
    Purge,
    /// Move to working memory (high-priority injection).
    Promote,
}

impl MemoryAction {
    /// Iterator over all possible actions for Q-Value maximization.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![Self::Retain, Self::Purge, Self::Promote]
    }
}
