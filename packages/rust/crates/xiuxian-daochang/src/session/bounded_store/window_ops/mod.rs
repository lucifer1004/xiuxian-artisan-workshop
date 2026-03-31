mod read_ops;
mod write_ops;

use crate::session::ChatMessage;
use xiuxian_window::TurnSlot;

fn turn_slots_to_messages(slots: &[TurnSlot]) -> Vec<ChatMessage> {
    slots
        .iter()
        .map(|slot| ChatMessage {
            role: slot.role.clone(),
            content: Some(slot.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        })
        .collect()
}
