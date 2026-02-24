use omni_window::TurnSlot;
use serde::{Deserialize, Serialize};

use crate::session::{ChatMessage, SessionSummarySegment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContextStats {
    pub messages: usize,
    pub summary_segments: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContextSnapshotInfo {
    pub messages: usize,
    pub summary_segments: usize,
    pub saved_at_unix_ms: Option<u64>,
    pub saved_age_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionContextMode {
    Bounded,
    Unbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContextWindowInfo {
    pub mode: SessionContextMode,
    pub messages: usize,
    pub summary_segments: usize,
    pub window_turns: Option<usize>,
    pub window_slots: Option<usize>,
    pub total_tool_calls: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SessionContextBackupMetadata {
    pub(super) messages: usize,
    pub(super) summary_segments: usize,
    pub(super) saved_at_unix_ms: u64,
}

#[derive(Clone, Default)]
pub(super) struct SessionContextBackup {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) summary_segments: Vec<SessionSummarySegment>,
    pub(super) window_slots: Vec<TurnSlot>,
}

impl SessionContextBackup {
    pub(super) fn stats(&self) -> SessionContextStats {
        let messages = if self.window_slots.is_empty() {
            self.messages.len()
        } else {
            self.window_slots.len()
        };
        SessionContextStats {
            messages,
            summary_segments: self.summary_segments.len(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.summary_segments.is_empty() && self.window_slots.is_empty()
    }
}
