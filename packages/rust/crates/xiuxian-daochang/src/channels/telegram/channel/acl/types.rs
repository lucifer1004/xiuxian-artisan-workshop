use crate::channels::control_command_authorization::ControlCommandPolicy;
use crate::channels::telegram::channel::TelegramSlashCommandRule;
use crate::channels::telegram::channel::admin_rules::TelegramCommandAdminRule;
use crate::channels::telegram::channel::group_policy::TelegramGroupPolicyConfig;

pub(super) const TELEGRAM_ACL_FIELD_ALLOWED_USERS: &str = "telegram.acl.allow.users";
pub(super) const TELEGRAM_ACL_FIELD_ADMIN_COMMAND_RULES: &str = "telegram.acl.control.rules";
pub(super) const TELEGRAM_ACL_FIELD_GROUP_ALLOW_FROM: &str = "telegram.group_allow_from";
pub(super) const TELEGRAM_ACL_FIELD_ADMIN_USERS: &str = "telegram.acl.admin.users";
pub(super) const TELEGRAM_ACL_FIELD_CONTROL_COMMAND_ALLOW_FROM: &str =
    "telegram.acl.control.allow_from.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_COMMAND_ALLOW_FROM: &str =
    "telegram.acl.slash.global.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_SESSION_STATUS_ALLOW_FROM: &str =
    "telegram.acl.slash.session_status.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_SESSION_BUDGET_ALLOW_FROM: &str =
    "telegram.acl.slash.session_budget.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_SESSION_MEMORY_ALLOW_FROM: &str =
    "telegram.acl.slash.session_memory.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_SESSION_FEEDBACK_ALLOW_FROM: &str =
    "telegram.acl.slash.session_feedback.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_JOB_ALLOW_FROM: &str =
    "telegram.acl.slash.job_status.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_JOBS_ALLOW_FROM: &str =
    "telegram.acl.slash.jobs_summary.users";
pub(super) const TELEGRAM_ACL_FIELD_SLASH_BG_ALLOW_FROM: &str =
    "telegram.acl.slash.background_submit.users";

pub(in crate::channels::telegram::channel) struct TelegramAclConfig {
    pub(in crate::channels::telegram::channel) allowed_users: Vec<String>,
    pub(in crate::channels::telegram::channel) allowed_groups: Vec<String>,
    pub(in crate::channels::telegram::channel) control_command_policy:
        ControlCommandPolicy<TelegramCommandAdminRule>,
    pub(in crate::channels::telegram::channel) slash_command_policy:
        ControlCommandPolicy<TelegramSlashCommandRule>,
    pub(in crate::channels::telegram::channel) group_policy_config: TelegramGroupPolicyConfig,
    pub(in crate::channels::telegram::channel) session_admin_persist: bool,
}
