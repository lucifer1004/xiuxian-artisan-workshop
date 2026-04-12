use crate::channels::traits::{ChannelAttachment, ChannelMessage};

use super::channel::DiscordChannel;
use super::serenity_payload::parse_discord_ingress_payload;

impl DiscordChannel {
    fn is_allowed_identity(&self, identity: &str) -> bool {
        let normalized = Self::normalize_identity(identity);
        self.allowed_users
            .iter()
            .any(|entry| entry == "*" || entry == &normalized)
    }

    fn is_any_identity_allowed<'a, I>(&self, identities: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        identities
            .into_iter()
            .any(|identity| self.is_allowed_identity(identity))
    }

    fn is_guild_allowed(&self, guild_id: &str) -> bool {
        let normalized = guild_id.trim();
        self.allowed_guilds
            .iter()
            .any(|entry| entry == "*" || entry == normalized)
    }

    fn build_session_key(&self, scope: &str, channel_id: &str, user_identity: &str) -> String {
        self.session_partition()
            .build_session_key(scope, channel_id, user_identity)
    }

    fn build_acl_identities(
        author_id: &str,
        username: Option<&str>,
        author_role_ids: &[String],
    ) -> Vec<String> {
        let mut identities = vec![Self::normalize_identity(author_id)];
        if let Some(name) = username {
            let normalized_name = Self::normalize_identity(name);
            if !normalized_name.is_empty() {
                identities.push(normalized_name);
            }
        }
        identities.extend(
            author_role_ids
                .iter()
                .map(|role_id| Self::normalize_identity(&format!("role:{role_id}"))),
        );

        let mut deduped = Vec::new();
        for identity in identities {
            if identity.is_empty() {
                continue;
            }
            if !deduped.iter().any(|existing| existing == &identity) {
                deduped.push(identity);
            }
        }
        deduped
    }

    /// Parse a Discord ingress payload into a channel message.
    ///
    /// Supported shapes (subset):
    /// - gateway-style message payload (`id`, `content`, `channel_id`, optional `guild_id`,
    ///   `author.id`)
    /// - slash command interaction payload (`type=2`) normalized to command text (for example
    ///   `/session memory json`).
    pub fn parse_gateway_message(&self, event: &serde_json::Value) -> Option<ChannelMessage> {
        let payload = parse_discord_ingress_payload(event)?;
        let message_id = payload.event_id;
        let is_interaction = payload.is_interaction;
        let mut text = payload.content;
        let attachments = map_discord_attachments(payload.attachments);
        if text.trim().is_empty() {
            if attachments.is_empty() {
                return None;
            }
            text = "[discord-image]".to_string();
        }

        let channel_id = payload.channel_id.to_string();
        let guild_id = payload.guild_id.as_ref().map(ToString::to_string);
        let author_id = payload.author_id.to_string();
        let username = payload.author_username.as_deref();
        let author_role_ids = payload.author_role_ids;
        let message_mentions_bot = payload
            .mentioned_user_ids
            .iter()
            .any(|mentioned_id| self.is_bot_user_id(Some(mentioned_id.as_str())));
        let reply_mentions_bot =
            self.is_bot_user_id(payload.referenced_message_author_id.as_deref());
        let acl_identities = Self::build_acl_identities(&author_id, username, &author_role_ids);

        let allowed_by_guild = guild_id
            .as_deref()
            .is_some_and(|id| self.is_guild_allowed(id));
        let allowed_by_user =
            self.is_any_identity_allowed(acl_identities.iter().map(String::as_str));

        if !allowed_by_guild && !allowed_by_user {
            tracing::warn!(
                "Discord: ignoring message from unauthorized sender (user_id={}, username={}, guild_id={}, channel_id={})",
                author_id,
                username.unwrap_or("(not set)"),
                guild_id.as_deref().unwrap_or("(dm)"),
                channel_id
            );
            return None;
        }

        if guild_id.is_some()
            && !is_interaction
            && !self.guild_message_passes_mention_policy(
                &channel_id,
                message_mentions_bot,
                reply_mentions_bot,
                is_command_like_trigger(&text),
            )
        {
            tracing::debug!(
                event = "discord.gateway.message.ignored.require_mention",
                message_id = %message_id,
                channel_id = %channel_id,
                guild_id = %guild_id.as_deref().unwrap_or_default(),
                author_id = %author_id,
                mention_trigger = message_mentions_bot,
                reply_trigger = reply_mentions_bot,
                command_trigger = is_command_like_trigger(&text),
                "discord guild message ignored: require_mention enabled and no trigger detected"
            );
            return None;
        }

        let scope = guild_id.as_deref().unwrap_or("dm");
        let sender = Self::normalize_identity(&author_id);
        let session_key = self.build_session_key(scope, &channel_id, &sender);
        self.cache_sender_acl_identities(&sender, &channel_id, acl_identities);

        Some(ChannelMessage {
            id: format!("discord_{channel_id}_{message_id}"),
            sender,
            recipient: channel_id.clone(),
            session_key,
            content: text,
            attachments,
            channel: "discord".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }
}

fn is_command_like_trigger(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

fn map_discord_attachments(
    payload: Vec<super::serenity_payload::DiscordIngressAttachmentPayload>,
) -> Vec<ChannelAttachment> {
    payload
        .into_iter()
        .filter_map(|attachment| {
            if !looks_like_image_attachment(
                attachment.url.as_str(),
                attachment.content_type.as_deref(),
            ) {
                return None;
            }
            Some(ChannelAttachment::ImageUrl {
                url: attachment.url,
            })
        })
        .collect()
}

fn looks_like_image_attachment(url: &str, content_type: Option<&str>) -> bool {
    if let Some(kind) = content_type
        && kind.trim().to_ascii_lowercase().starts_with("image/")
    {
        return true;
    }

    let lower = url.to_ascii_lowercase();
    [
        ".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp", ".tiff", ".svg", ".heic",
    ]
    .iter()
    .any(|ext| lower.contains(ext))
}
