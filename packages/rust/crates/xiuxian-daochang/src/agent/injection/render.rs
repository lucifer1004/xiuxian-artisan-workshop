//! Render module for converting injection snapshots to chat messages.

use xiuxian_qianhuan::InjectionSnapshot;

use crate::session::ChatMessage;

/// Render an injection snapshot into chat messages.
pub(super) fn render_snapshot_messages(snapshot: &InjectionSnapshot) -> Vec<ChatMessage> {
    snapshot
        .blocks
        .iter()
        .map(|block| ChatMessage {
            role: "system".to_string(),
            content: Some(block.payload.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        })
        .collect()
}
