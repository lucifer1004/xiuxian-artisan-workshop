use crate::channels::managed_runtime::parsing::SessionMentionMode;
use crate::channels::traits::RecipientMentionPolicyStatus;

fn format_override(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "on",
        Some(false) => "off",
        None => "inherit",
    }
}

fn format_effective(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

pub(in super::super::super) fn format_session_mention_status(
    recipient: &str,
    status: &RecipientMentionPolicyStatus,
) -> String {
    format!(
        "Session mention policy.\nrecipient={recipient}\ndefault_require_mention={}\noverride={}\neffective_require_mention={}\npersist_enabled={}",
        format_effective(status.default_require_mention),
        format_override(status.recipient_override),
        format_effective(status.effective_require_mention),
        status.persist_enabled,
    )
}

pub(in super::super::super) fn format_session_mention_updated(
    recipient: &str,
    mode: SessionMentionMode,
    status: &RecipientMentionPolicyStatus,
) -> String {
    let action = match mode {
        SessionMentionMode::Require => "on",
        SessionMentionMode::Open => "off",
        SessionMentionMode::Inherit => "inherit",
    };
    format!(
        "Session mention policy updated.\nrecipient={recipient}\naction={action}\ndefault_require_mention={}\noverride={}\neffective_require_mention={}\npersist_enabled={}",
        format_effective(status.default_require_mention),
        format_override(status.recipient_override),
        format_effective(status.effective_require_mention),
        status.persist_enabled,
    )
}

pub(in super::super::super) fn format_session_mention_admin_required(
    sender: &str,
    recipient: &str,
    status: &RecipientMentionPolicyStatus,
) -> String {
    format!(
        "Admin permission required for `/session mention`.\nsender={sender}\nrecipient={recipient}\neffective_require_mention={}\noverride={}",
        format_effective(status.effective_require_mention),
        format_override(status.recipient_override),
    )
}
