use std::sync::PoisonError;

use crate::channels::traits::RecipientCommandAdminUsersMutation;

use super::auth::normalize_discord_identity;
use super::state::DiscordChannel;

impl DiscordChannel {
    pub(super) fn recipient_override_admin_users(
        &self,
        recipient: &str,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let recipient_key = normalize_recipient(recipient)?;
        let overrides = self
            .recipient_admin_users
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        Ok(overrides.get(&recipient_key).cloned())
    }

    pub(super) fn mutate_recipient_override_admin_users(
        &self,
        recipient: &str,
        mutation: RecipientCommandAdminUsersMutation,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let recipient_key = normalize_recipient(recipient)?;
        let mut overrides = self
            .recipient_admin_users
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let current = overrides.get(&recipient_key).cloned();

        let next = match mutation {
            RecipientCommandAdminUsersMutation::Clear => None,
            RecipientCommandAdminUsersMutation::Set(entries) => {
                Some(normalize_admin_user_mutation_entries(entries)?)
            }
            RecipientCommandAdminUsersMutation::Add(entries) => {
                let mut merged = current.unwrap_or_default();
                merged.extend(normalize_admin_user_mutation_entries(entries)?);
                Some(dedup_preserve_order(merged))
            }
            RecipientCommandAdminUsersMutation::Remove(entries) => {
                let removals = normalize_admin_user_mutation_entries(entries)?;
                let Some(existing) = current else {
                    return Ok(None);
                };
                let filtered: Vec<String> = existing
                    .into_iter()
                    .filter(|entry| !removals.iter().any(|removal| removal == entry))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(dedup_preserve_order(filtered))
                }
            }
        };

        match next.clone() {
            Some(entries) => {
                overrides.insert(recipient_key, entries);
            }
            None => {
                overrides.remove(&recipient_key);
            }
        }
        Ok(next)
    }

    pub(super) fn resolve_recipient_command_admin_users(
        &self,
        recipient: &str,
    ) -> Option<Vec<String>> {
        let recipient_key = recipient.trim();
        if recipient_key.is_empty() {
            return None;
        }
        self.recipient_admin_users
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(recipient_key)
            .cloned()
    }
}

fn normalize_recipient(recipient: &str) -> anyhow::Result<String> {
    let normalized = recipient.trim();
    if normalized.is_empty() {
        return Err(anyhow::anyhow!(
            "recipient-scoped admin override requires a non-empty recipient key"
        ));
    }
    Ok(normalized.to_string())
}

fn normalize_admin_user_mutation_entries(entries: Vec<String>) -> anyhow::Result<Vec<String>> {
    let normalized = dedup_preserve_order(
        entries
            .into_iter()
            .map(|entry| normalize_discord_identity(&entry))
            .filter(|entry| !entry.is_empty())
            .collect(),
    );
    if normalized.is_empty() {
        return Err(anyhow::anyhow!(
            "no valid Discord identities provided for recipient-scoped admin override"
        ));
    }
    Ok(normalized)
}

fn dedup_preserve_order(entries: Vec<String>) -> Vec<String> {
    let mut deduped: Vec<String> = Vec::new();
    for entry in entries {
        if !deduped.iter().any(|existing| existing == &entry) {
            deduped.push(entry);
        }
    }
    deduped
}
