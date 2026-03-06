mod backup;
mod types;
mod window_ops;

use std::time::{SystemTime, UNIX_EPOCH};

use super::Agent;
use crate::session::{ChatMessage, SessionSummarySegment};

pub use types::{
    SessionContextMode, SessionContextSnapshotInfo, SessionContextStats, SessionContextWindowInfo,
};

const SESSION_CONTEXT_BACKUP_PREFIX: &str = "__session_context_backup__:";
const SESSION_CONTEXT_BACKUP_META_PREFIX: &str = "__session_context_backup_meta__:";

pub(super) fn backup_session_id(session_id: &str) -> String {
    format!("{SESSION_CONTEXT_BACKUP_PREFIX}{session_id}")
}

pub(super) fn backup_metadata_session_id(session_id: &str) -> String {
    format!("{SESSION_CONTEXT_BACKUP_META_PREFIX}{session_id}")
}

pub(super) fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[must_use]
pub(crate) fn test_now_unix_ms() -> u64 {
    now_unix_ms()
}

impl Agent {
    pub(crate) fn test_set_session_reset_idle_timeout_ms(&mut self, timeout_ms: Option<u64>) {
        self.session_reset_idle_timeout_ms = timeout_ms;
    }

    pub(crate) async fn test_set_session_last_activity(&self, session_id: &str, unix_ms: u64) {
        self.session_last_activity_unix_ms
            .write()
            .await
            .insert(session_id.to_string(), unix_ms);
    }

    pub(crate) async fn test_enforce_session_reset_policy(
        &self,
        session_id: &str,
    ) -> anyhow::Result<()> {
        self.enforce_session_reset_policy(session_id).await
    }

    pub(crate) async fn test_session_messages(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<ChatMessage>> {
        self.session.get(session_id).await
    }

    pub(crate) async fn test_bounded_recent_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<ChatMessage>> {
        if let Some(ref bounded) = self.bounded_session {
            bounded.get_recent_messages(session_id, limit).await
        } else {
            Ok(Vec::new())
        }
    }

    pub(crate) async fn test_bounded_recent_summary_segments(
        &self,
        session_id: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<SessionSummarySegment>> {
        if let Some(ref bounded) = self.bounded_session {
            bounded.get_recent_summary_segments(session_id, limit).await
        } else {
            Ok(Vec::new())
        }
    }
}
