mod agent_ops;
mod storage;
mod types;

use super::Agent;
use crate::ChatMessage;

pub(crate) use types::SessionMemoryRecallSnapshotInput;
pub(crate) use types::{
    EMBEDDING_SOURCE_EMBEDDING, EMBEDDING_SOURCE_EMBEDDING_REPAIRED, EMBEDDING_SOURCE_UNKNOWN,
};
pub use types::{SessionMemoryRecallDecision, SessionMemoryRecallSnapshot};

pub(crate) fn test_snapshot_session_id(session_id: &str) -> String {
    storage::snapshot_session_id(session_id)
}

impl Agent {
    pub(crate) async fn test_record_memory_recall_snapshot(
        &self,
        session_id: &str,
        snapshot: SessionMemoryRecallSnapshot,
    ) {
        self.record_memory_recall_snapshot(session_id, snapshot)
            .await;
    }

    pub(crate) async fn test_append_memory_recall_snapshot_payload(
        &self,
        session_id: &str,
        payload: String,
    ) -> anyhow::Result<()> {
        let storage_session_id = storage::snapshot_session_id(session_id);
        self.session
            .append(
                &storage_session_id,
                vec![ChatMessage {
                    role: "system".to_string(),
                    content: Some(payload),
                    tool_calls: None,
                    tool_call_id: None,
                    name: Some(storage::MEMORY_RECALL_SNAPSHOT_MESSAGE_NAME.to_string()),
                }],
            )
            .await
    }
}
