use crate::channels::control_command_authorization::ControlCommandAuthRule;
use crate::channels::control_command_rule_specs::CommandSelectorAuthRule;

pub type DiscordCommandAdminRule = CommandSelectorAuthRule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscordSlashCommandRule {
    pub(super) command_scope: &'static str,
    pub(super) allowed_identities: Vec<String>,
}

impl DiscordSlashCommandRule {
    pub(super) fn new(command_scope: &'static str, allowed_identities: Vec<String>) -> Self {
        Self {
            command_scope,
            allowed_identities,
        }
    }
}

impl ControlCommandAuthRule for DiscordSlashCommandRule {
    fn matches(&self, command_text: &str) -> bool {
        self.command_scope == command_text
    }

    fn allows_identity(&self, identity: &str) -> bool {
        self.allowed_identities
            .iter()
            .any(|entry| entry == "*" || entry == identity)
    }
}

/// Authorization inputs for privileged Discord control commands.
#[derive(Debug, Clone, Default)]
pub struct DiscordControlCommandPolicy {
    pub admin_users: Vec<String>,
    pub control_command_allow_from: Option<Vec<String>>,
    pub control_command_rules: Vec<DiscordCommandAdminRule>,
    pub slash_command_policy: DiscordSlashCommandPolicy,
}

/// User-friendly ACL fields for non-privileged Discord slash commands.
///
/// Priority order:
/// 1) `slash_command_allow_from` (global override for all listed slash scopes)
/// 2) command-specific allowlists (`*_allow_from`)
/// 3) fallback `admin_users` from [`DiscordControlCommandPolicy`]
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_field_names)]
pub struct DiscordSlashCommandPolicy {
    pub slash_command_allow_from: Option<Vec<String>>,
    pub session_status_allow_from: Option<Vec<String>>,
    pub session_budget_allow_from: Option<Vec<String>>,
    pub session_memory_allow_from: Option<Vec<String>>,
    pub session_feedback_allow_from: Option<Vec<String>>,
    pub job_status_allow_from: Option<Vec<String>>,
    pub jobs_summary_allow_from: Option<Vec<String>>,
    pub background_submit_allow_from: Option<Vec<String>>,
}

impl DiscordControlCommandPolicy {
    #[must_use]
    pub fn new(
        admin_users: Vec<String>,
        control_command_allow_from: Option<Vec<String>>,
        control_command_rules: Vec<DiscordCommandAdminRule>,
    ) -> Self {
        Self {
            admin_users,
            control_command_allow_from,
            control_command_rules,
            slash_command_policy: DiscordSlashCommandPolicy::default(),
        }
    }

    #[must_use]
    pub fn with_slash_command_policy(
        mut self,
        slash_command_policy: DiscordSlashCommandPolicy,
    ) -> Self {
        self.slash_command_policy = slash_command_policy;
        self
    }
}
