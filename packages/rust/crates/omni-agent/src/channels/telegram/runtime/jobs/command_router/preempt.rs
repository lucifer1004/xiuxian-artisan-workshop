use crate::channels::telegram::runtime::dispatch::ForegroundInterruptController;
use crate::channels::traits::ChannelMessage;

pub(super) fn interrupt_active_turn_for_new_message(
    interrupt_controller: &ForegroundInterruptController,
    session_id: &str,
    msg: &ChannelMessage,
) {
    if interrupt_controller.interrupt(session_id) {
        tracing::info!(
            event = "telegram.foreground.turn.preempted",
            session_key = %msg.session_key,
            channel = %msg.channel,
            recipient = %msg.recipient,
            sender = %msg.sender,
            "active foreground turn interrupted by newer inbound message"
        );
    }
}
