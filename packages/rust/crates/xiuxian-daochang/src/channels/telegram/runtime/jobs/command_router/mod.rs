mod background;
mod dispatch;
mod foreground;
mod preempt;
mod session;

pub(in crate::channels::telegram::runtime::jobs) use dispatch::handle_inbound_message;
