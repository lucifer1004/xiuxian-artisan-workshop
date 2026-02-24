use std::sync::RwLock;

use crate::channels::control_command_authorization::ControlCommandPolicy;
use crate::config::runtime_settings_paths;

use super::super::TelegramSessionPartition;
use super::super::acl::{
    normalize_allowed_group_entries, normalize_allowed_user_entries_with_context,
    normalize_control_command_policy, normalize_slash_command_policy,
};
use super::super::acl_reload::TelegramAclReloadState;
use super::super::admin_rules::TelegramCommandAdminRule;
use super::super::group_policy::TelegramGroupPolicyConfig;
use super::super::policy::TelegramSlashCommandRule;
use super::super::send_gate::{TelegramSendRateLimitBackend, TelegramSendRateLimitGateState};
use super::super::state::TelegramChannel;

impl TelegramChannel {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new_with_base_url_and_partition_and_client_impl(
        bot_token: String,
        allowed_users: Vec<String>,
        allowed_groups: Vec<String>,
        api_base_url: String,
        control_command_policy: ControlCommandPolicy<TelegramCommandAdminRule>,
        slash_command_policy: ControlCommandPolicy<TelegramSlashCommandRule>,
        session_partition: TelegramSessionPartition,
        client: reqwest::Client,
    ) -> Self {
        let (system_settings_path, user_settings_path) = runtime_settings_paths();
        let control_command_policy = normalize_control_command_policy(control_command_policy);
        let slash_command_policy = normalize_slash_command_policy(slash_command_policy);
        Self {
            bot_token,
            api_base_url,
            allowed_users: RwLock::new(normalize_allowed_user_entries_with_context(
                allowed_users,
                "telegram.acl.allow.users",
            )),
            allowed_groups: RwLock::new(normalize_allowed_group_entries(allowed_groups)),
            control_command_policy: RwLock::new(control_command_policy),
            slash_command_policy: RwLock::new(slash_command_policy),
            group_policy_config: RwLock::new(TelegramGroupPolicyConfig::default()),
            session_admin_persist: RwLock::new(false),
            session_partition: RwLock::new(session_partition),
            acl_reload_state: RwLock::new(TelegramAclReloadState::new(
                system_settings_path,
                user_settings_path,
            )),
            send_rate_limit_gate: tokio::sync::Mutex::new(TelegramSendRateLimitGateState::default()),
            send_rate_limit_backend: TelegramSendRateLimitBackend::from_env(),
            client,
        }
    }

    #[doc(hidden)]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new_with_base_url_and_send_rate_limit_valkey_for_test(
        bot_token: String,
        allowed_users: Vec<String>,
        allowed_groups: Vec<String>,
        api_base_url: String,
        redis_url: String,
        key_prefix: String,
    ) -> anyhow::Result<Self> {
        let mut channel =
            Self::new_with_base_url(bot_token, allowed_users, allowed_groups, api_base_url);
        channel.send_rate_limit_backend = TelegramSendRateLimitBackend::new_valkey_for_test(
            redis_url.as_str(),
            key_prefix.as_str(),
        )?;
        Ok(channel)
    }
}
