use std::collections::HashMap;
use std::sync::{PoisonError, RwLock};

use crate::channels::control_command_authorization::ControlCommandPolicy;

use super::super::session_partition::DiscordSessionPartition;
use super::policy::{DiscordCommandAdminRule, DiscordSlashCommandRule};

/// Discord channel skeleton.
///
/// This type currently provides:
/// - channel identity/allowlist configuration storage
/// - session partition and parser support for future transport integration
/// - shared control-command authorization policy resolution
/// - Discord REST send path (`send`) and typing indicator API call (`start_typing`)
///
/// `listen` remains intentionally unimplemented in this phase.
pub struct DiscordChannel {
    pub(in super::super) bot_token: String,
    pub(super) api_base_url: String,
    pub(in super::super) allowed_users: Vec<String>,
    pub(in super::super) allowed_guilds: Vec<String>,
    pub(super) control_command_policy: ControlCommandPolicy<DiscordCommandAdminRule>,
    pub(super) slash_command_policy: ControlCommandPolicy<DiscordSlashCommandRule>,
    pub(super) session_partition: RwLock<DiscordSessionPartition>,
    pub(super) recipient_admin_users: RwLock<HashMap<String, Vec<String>>>,
    pub(super) sender_acl_identities: RwLock<HashMap<String, Vec<String>>>,
    pub(in super::super) client: reqwest::Client,
}

impl DiscordChannel {
    /// Current session partition mode used by this channel.
    pub fn session_partition(&self) -> DiscordSessionPartition {
        *self
            .session_partition
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Update session partition mode at runtime.
    pub fn set_session_partition(&self, mode: DiscordSessionPartition) {
        *self
            .session_partition
            .write()
            .unwrap_or_else(PoisonError::into_inner) = mode;
    }
}
