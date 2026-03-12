//! Render module for converting injection snapshots to chat messages.

use xiuxian_qianhuan::InjectionSnapshot;

use crate::session::ChatMessage;

/// Render an injection snapshot into chat messages.
pub(super) fn render_snapshot_messages(snapshot: &InjectionSnapshot) -> Vec<ChatMessage> {
    // Placeholder implementation - convert snapshot blocks to messages
    let mut messages = Vec::new();

    for block in &snapshot.blocks {
        // Create a user message from each block
        messages.push(ChatMessage::user(block.content.clone()));
    }

    messages
}
