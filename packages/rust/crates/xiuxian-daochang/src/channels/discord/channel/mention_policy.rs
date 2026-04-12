use std::collections::HashMap;
use std::sync::PoisonError;

use crate::channels::traits::RecipientMentionPolicyStatus;
use crate::config::runtime_settings_paths;

use super::mention_policy_persistence::persist_recipient_require_mention_to_user_settings;
use super::state::DiscordChannel;

impl DiscordChannel {
    pub(in super::super) fn configure_mention_policy(
        &self,
        default_require_mention: bool,
        persist_enabled: bool,
        recipient_overrides: HashMap<String, bool>,
    ) {
        *self
            .default_require_mention
            .write()
            .unwrap_or_else(PoisonError::into_inner) = default_require_mention;
        *self
            .require_mention_persist
            .write()
            .unwrap_or_else(PoisonError::into_inner) = persist_enabled;
        *self
            .recipient_require_mention
            .write()
            .unwrap_or_else(PoisonError::into_inner) = recipient_overrides
            .into_iter()
            .filter_map(|(recipient, require_mention)| {
                let normalized = recipient.trim().to_string();
                (!normalized.is_empty()).then_some((normalized, require_mention))
            })
            .collect();
    }

    pub(in super::super) fn default_require_mention(&self) -> bool {
        *self
            .default_require_mention
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(in super::super) fn require_mention_persist_enabled(&self) -> bool {
        *self
            .require_mention_persist
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(in super::super) fn explicit_recipient_require_mention(
        &self,
        recipient: &str,
    ) -> Option<bool> {
        let normalized = recipient.trim();
        if normalized.is_empty() {
            return None;
        }
        let overrides = self
            .recipient_require_mention
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        overrides
            .get(normalized)
            .copied()
            .or_else(|| overrides.get("*").copied())
    }

    pub(in super::super) fn effective_require_mention_for_recipient(
        &self,
        recipient: &str,
    ) -> bool {
        self.explicit_recipient_require_mention(recipient)
            .unwrap_or_else(|| self.default_require_mention())
    }

    pub(in super::super) fn recipient_mention_policy_status(
        &self,
        recipient: &str,
    ) -> anyhow::Result<RecipientMentionPolicyStatus> {
        validate_recipient(recipient)?;
        let default_require_mention = self.default_require_mention();
        let recipient_override = self.explicit_recipient_override_only(recipient)?;
        Ok(RecipientMentionPolicyStatus {
            default_require_mention,
            recipient_override,
            effective_require_mention: recipient_override.unwrap_or(default_require_mention),
            persist_enabled: self.require_mention_persist_enabled(),
        })
    }

    pub(in super::super) fn set_recipient_require_mention(
        &self,
        recipient: &str,
        require_mention: Option<bool>,
    ) -> anyhow::Result<RecipientMentionPolicyStatus> {
        let normalized_recipient = validate_recipient(recipient)?;
        {
            let mut overrides = self
                .recipient_require_mention
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            match require_mention {
                Some(value) => {
                    overrides.insert(normalized_recipient.clone(), value);
                }
                None => {
                    overrides.remove(&normalized_recipient);
                }
            }
        }

        if self.require_mention_persist_enabled() {
            let (_, user_settings_path) = runtime_settings_paths();
            persist_recipient_require_mention_to_user_settings(
                user_settings_path.as_path(),
                &normalized_recipient,
                require_mention,
            )?;
        }

        self.recipient_mention_policy_status(&normalized_recipient)
    }

    pub(in super::super) fn guild_message_passes_mention_policy(
        &self,
        recipient: &str,
        message_mentions_bot: bool,
        reply_mentions_bot: bool,
        command_like_trigger: bool,
    ) -> bool {
        if !self.effective_require_mention_for_recipient(recipient) {
            return true;
        }
        command_like_trigger || message_mentions_bot || reply_mentions_bot
    }

    #[doc(hidden)]
    pub fn configure_mention_policy_for_tests(
        &self,
        default_require_mention: bool,
        recipient_overrides: HashMap<String, bool>,
    ) {
        self.configure_mention_policy(default_require_mention, false, recipient_overrides);
    }

    fn explicit_recipient_override_only(&self, recipient: &str) -> anyhow::Result<Option<bool>> {
        let normalized = validate_recipient(recipient)?;
        Ok(self
            .recipient_require_mention
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&normalized)
            .copied())
    }
}

fn validate_recipient(recipient: &str) -> anyhow::Result<String> {
    let normalized = recipient.trim();
    if normalized.is_empty() {
        return Err(anyhow::anyhow!(
            "recipient-scoped mention policy requires a non-empty recipient key"
        ));
    }
    Ok(normalized.to_string())
}
