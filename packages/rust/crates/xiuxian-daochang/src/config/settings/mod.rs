//! Runtime settings loader for omni-agent.
//!
//! Loads and merges:
//! - System defaults: `<PRJ_ROOT>/packages/conf/xiuxian.toml`
//! - User overrides:  `<PRJ_CONFIG_HOME>/xiuxian-artisan-workshop/xiuxian.toml`
//!
//! Merge precedence is user over system.

mod loader;
mod merge;
mod types;

pub use loader::{
    load_runtime_settings, load_runtime_settings_from_paths, runtime_settings_paths,
    set_config_home_override,
};
pub use types::{
    DiscordAclAllowSettings, DiscordAclControlSettings, DiscordAclPrincipalSettings,
    DiscordAclSettings, DiscordAclSlashSettings, DiscordSettings, EmbeddingSettings,
    InferenceSettings, MemorySettings, MistralSettings, RuntimeSettings, SessionSettings,
    TelegramAclAllowSettings, TelegramAclControlSettings, TelegramAclPrincipalSettings,
    TelegramAclRuleSettings, TelegramAclSettings, TelegramAclSlashSettings, TelegramGroupSettings,
    TelegramSettings, TelegramTopicSettings, ToolRuntimeSettings,
};
