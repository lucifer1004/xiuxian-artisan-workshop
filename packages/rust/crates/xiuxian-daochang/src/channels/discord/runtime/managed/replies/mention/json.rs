use serde_json::json;

use crate::channels::managed_runtime::parsing::SessionMentionMode;
use crate::channels::traits::RecipientMentionPolicyStatus;

fn action_label(mode: SessionMentionMode) -> &'static str {
    match mode {
        SessionMentionMode::Require => "on",
        SessionMentionMode::Open => "off",
        SessionMentionMode::Inherit => "inherit",
    }
}

pub(in super::super::super) fn format_session_mention_status_json(
    recipient: &str,
    status: &RecipientMentionPolicyStatus,
) -> String {
    json!({
        "kind": "session_mention",
        "recipient": recipient,
        "default_require_mention": status.default_require_mention,
        "recipient_override": status.recipient_override,
        "effective_require_mention": status.effective_require_mention,
        "persist_enabled": status.persist_enabled,
    })
    .to_string()
}

pub(in super::super::super) fn format_session_mention_updated_json(
    recipient: &str,
    mode: SessionMentionMode,
    status: &RecipientMentionPolicyStatus,
) -> String {
    json!({
        "kind": "session_mention",
        "recipient": recipient,
        "updated": true,
        "action": action_label(mode),
        "default_require_mention": status.default_require_mention,
        "recipient_override": status.recipient_override,
        "effective_require_mention": status.effective_require_mention,
        "persist_enabled": status.persist_enabled,
    })
    .to_string()
}

pub(in super::super::super) fn format_session_mention_admin_required_json(
    sender: &str,
    recipient: &str,
    status: &RecipientMentionPolicyStatus,
) -> String {
    json!({
        "kind": "session_mention",
        "status": "admin_required",
        "sender": sender,
        "recipient": recipient,
        "default_require_mention": status.default_require_mention,
        "recipient_override": status.recipient_override,
        "effective_require_mention": status.effective_require_mention,
        "persist_enabled": status.persist_enabled,
    })
    .to_string()
}
