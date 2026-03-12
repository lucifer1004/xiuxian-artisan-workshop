//! Session namespace: message types, session store, and optional bounded session store.

mod bounded_store;
mod message;
mod store;

pub use bounded_store::BoundedSessionStore;
pub use message::{ChatMessage, FunctionCall, ToolCallOut};
pub use store::SessionStore;
