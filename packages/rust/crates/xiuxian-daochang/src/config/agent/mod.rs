//! Agent configuration: inference API, model, API key, and external tool server list.

mod agent_defaults;
mod memory_defaults;
mod types;

pub use agent_defaults::LITELLM_DEFAULT_URL;
pub use types::{AgentConfig, ContextBudgetStrategy, MemoryConfig, ToolServerEntry};
