mod backup;
mod types;
mod window_ops;

use std::time::{SystemTime, UNIX_EPOCH};

use super::Agent;

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
