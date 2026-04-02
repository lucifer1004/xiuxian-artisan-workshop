//! Pluggable notification dispatch for agent-side reminders.

mod dispatcher;
mod provider;

pub use dispatcher::NotificationDispatcher;
pub use provider::NotificationProvider;
