//! Config namespace: agent config and external tool config loading.

mod agent;
mod settings;
mod tools;
mod xiuxian;

pub use agent::{
    AgentConfig, ContextBudgetStrategy, LITELLM_DEFAULT_URL, MemoryConfig, ToolServerEntry,
};
pub use settings::{
    DiscordAclAllowSettings, DiscordAclControlSettings, DiscordAclPrincipalSettings,
    DiscordAclSettings, DiscordAclSlashSettings, DiscordSettings, EmbeddingSettings,
    MemorySettings, RuntimeSettings, SessionSettings, TelegramAclAllowSettings,
    TelegramAclControlSettings, TelegramAclPrincipalSettings, TelegramAclSettings,
    TelegramAclSlashSettings, TelegramGroupSettings, TelegramSettings, TelegramTopicSettings,
    ToolRuntimeSettings, load_runtime_settings, load_runtime_settings_from_paths,
    runtime_settings_paths, set_config_home_override,
};
pub use tools::{ToolConfigFile, ToolServerEntryFile, load_tool_config};
pub use xiuxian::{
    XiuxianConfig, load_xiuxian_config, load_xiuxian_config_from_bases,
    load_xiuxian_config_from_paths,
};
