mod background;
mod dispatch;
mod foreground;
mod preempt;
mod session;

#[cfg(test)]
pub(in crate::channels::telegram::runtime::jobs) use dispatch::handle_inbound_message;
pub(in crate::channels::telegram::runtime::jobs) use dispatch::handle_inbound_message_with_interrupt;
