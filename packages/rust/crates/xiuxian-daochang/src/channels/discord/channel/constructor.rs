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
            client: crate::channels::discord::client::build_discord_http_client(),
        }
    }
}
