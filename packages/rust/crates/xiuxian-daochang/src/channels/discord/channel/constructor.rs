use std::collections::HashMap;
use std::sync::RwLock;

use crate::channels::control_command_authorization::ControlCommandPolicy;
use crate::channels::discord::constants::DISCORD_DEFAULT_API_BASE;
use crate::channels::discord::session_partition::DiscordSessionPartition;

use super::policy::DiscordControlCommandPolicy;
use super::policy_builders::{
    build_slash_command_policy, normalize_allowed_guild_entries, normalize_allowed_user_entries,
};
use super::state::DiscordChannel;

impl DiscordChannel {
    #[must_use]
    /// Builds a Discord channel with the default control-command policy and the
    /// default `GuildChannelUser` session partition strategy.
    pub fn new(bot_token: String, allowed_users: Vec<String>, allowed_guilds: Vec<String>) -> Self {
        Self::new_with_partition_and_control_command_policy(
            bot_token,
            allowed_users,
            allowed_guilds,
            DiscordControlCommandPolicy::default(),
            DiscordSessionPartition::GuildChannelUser,
        )
    }

    #[must_use]
    /// Builds a Discord channel with an explicit session partition strategy.
    pub fn new_with_partition(
        bot_token: String,
        allowed_users: Vec<String>,
        allowed_guilds: Vec<String>,
        session_partition: DiscordSessionPartition,
    ) -> Self {
        Self::new_with_partition_and_control_command_policy(
            bot_token,
            allowed_users,
            allowed_guilds,
            DiscordControlCommandPolicy::default(),
            session_partition,
        )
    }

    #[must_use]
    /// Builds a Discord channel with an explicit control-command policy.
    pub fn new_with_control_command_policy(
        bot_token: String,
        allowed_users: Vec<String>,
        allowed_guilds: Vec<String>,
        control_command_policy: DiscordControlCommandPolicy,
    ) -> Self {
        Self::new_with_partition_and_control_command_policy(
            bot_token,
            allowed_users,
            allowed_guilds,
            control_command_policy,
            DiscordSessionPartition::GuildChannelUser,
        )
    }

    #[must_use]
    /// Builds a Discord channel and overrides the Discord HTTP API base URL.
    pub fn new_with_base_url(
        bot_token: String,
        allowed_users: Vec<String>,
        allowed_guilds: Vec<String>,
        api_base_url: String,
    ) -> Self {
        let mut channel = Self::new(bot_token, allowed_users, allowed_guilds);
        channel.api_base_url = api_base_url;
        channel
    }

    #[doc(hidden)]
    #[must_use]
    /// Compatibility helper used by older test call sites that chained
    /// `.expect(...)` on constructors.
    pub fn expect(self, _message: &str) -> Self {
        self
    }

    #[must_use]
    /// Builds a Discord channel with explicit session partition and control
    /// command policy settings.
    pub fn new_with_partition_and_control_command_policy(
        bot_token: String,
        allowed_users: Vec<String>,
        allowed_guilds: Vec<String>,
        control_command_policy: DiscordControlCommandPolicy,
        session_partition: DiscordSessionPartition,
    ) -> Self {
        let DiscordControlCommandPolicy {
            admin_users,
            control_command_allow_from,
            control_command_rules,
            slash_command_policy,
        } = control_command_policy;
        let normalized_admin_users = normalize_allowed_user_entries(admin_users);
        let slash_command_policy =
            build_slash_command_policy(normalized_admin_users.clone(), slash_command_policy);
        let control_command_policy = ControlCommandPolicy::new(
            normalized_admin_users,
            control_command_allow_from.map(normalize_allowed_user_entries),
            control_command_rules,
        );

        Self {
            bot_token,
            api_base_url: DISCORD_DEFAULT_API_BASE.to_string(),
            allowed_users: normalize_allowed_user_entries(allowed_users),
            allowed_guilds: normalize_allowed_guild_entries(allowed_guilds),
            control_command_policy,
            slash_command_policy,
            session_partition: RwLock::new(session_partition),
            recipient_admin_users: RwLock::new(HashMap::new()),
            sender_acl_identities: RwLock::new(HashMap::new()),
            bot_user_id: RwLock::new(None),
            default_require_mention: RwLock::new(false),
            require_mention_persist: RwLock::new(false),
            recipient_require_mention: RwLock::new(HashMap::new()),
            client: crate::channels::discord::client::build_discord_http_client(),
        }
    }
}
