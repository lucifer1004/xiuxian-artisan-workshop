use crate::channels::traits::ChannelMessage;

use super::super::TelegramChannel;
use super::types::ParsedTelegramUpdate;

const PHOTO_PLACEHOLDER_TEXT: &str = "[telegram-photo]";
const IMAGE_PLACEHOLDER_TEXT: &str = "[telegram-image]";
const VIDEO_PLACEHOLDER_TEXT: &str = "[telegram-video]";
const DOCUMENT_PLACEHOLDER_TEXT: &str = "[telegram-document]";
const VOICE_PLACEHOLDER_TEXT: &str = "[telegram-voice]";
const AUDIO_PLACEHOLDER_TEXT: &str = "[telegram-audio]";
const STICKER_PLACEHOLDER_TEXT: &str = "[telegram-sticker]";

fn extract_message_content_text(message: &serde_json::Value) -> Option<&str> {
    message
        .get("text")
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            message
                .get("caption")
                .and_then(serde_json::Value::as_str)
                .filter(|caption| !caption.trim().is_empty())
        })
        .or_else(|| {
            if message.get("photo").is_some() {
                Some(PHOTO_PLACEHOLDER_TEXT)
            } else if message.get("image").is_some() {
                Some(IMAGE_PLACEHOLDER_TEXT)
            } else if message.get("video").is_some() {
                Some(VIDEO_PLACEHOLDER_TEXT)
            } else if message.get("document").is_some() {
                Some(DOCUMENT_PLACEHOLDER_TEXT)
            } else if message.get("voice").is_some() {
                Some(VOICE_PLACEHOLDER_TEXT)
            } else if message.get("audio").is_some() {
                Some(AUDIO_PLACEHOLDER_TEXT)
            } else if message.get("sticker").is_some() {
                Some(STICKER_PLACEHOLDER_TEXT)
            } else {
                None
            }
        })
}

pub(super) fn extract_update_message(
    update: &serde_json::Value,
) -> Option<ParsedTelegramUpdate<'_>> {
    let message = update.get("message")?;
    let text = extract_message_content_text(message)?;
    let chat = message.get("chat")?;
    let chat_id = chat
        .get("id")
        .and_then(serde_json::Value::as_i64)
        .map(|id| id.to_string())?;
    let chat_title = chat
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(not set)");
    let chat_type = chat
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(not set)");
    let username = message
        .get("from")
        .and_then(|from| from.get("username"))
        .and_then(serde_json::Value::as_str);
    let user_id = message
        .get("from")
        .and_then(|from| from.get("id"))
        .and_then(serde_json::Value::as_i64)
        .map(|id| id.to_string());
    let message_thread_id = message
        .get("message_thread_id")
        .and_then(serde_json::Value::as_i64);
    let message_id = message
        .get("message_id")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let update_id = update
        .get("update_id")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();

    Some(ParsedTelegramUpdate {
        message,
        text,
        chat_id,
        chat_title,
        chat_type,
        username,
        user_id,
        message_thread_id,
        message_id,
        update_id,
    })
}

fn build_session_key(
    channel: &TelegramChannel,
    chat_id: &str,
    user_identity: &str,
    message_thread_id: Option<i64>,
) -> String {
    channel
        .session_partition()
        .build_session_key(chat_id, user_identity, message_thread_id)
}

pub(super) fn build_channel_message_from_parsed(
    channel: &TelegramChannel,
    parsed: &ParsedTelegramUpdate<'_>,
    user_identity: &str,
) -> ChannelMessage {
    let session_key = build_session_key(
        channel,
        &parsed.chat_id,
        user_identity,
        parsed.message_thread_id,
    );
    ChannelMessage {
        id: format!(
            "telegram_{}_{}_{}",
            parsed.chat_id, parsed.message_id, parsed.update_id
        ),
        sender: user_identity.to_string(),
        recipient: parsed.recipient(),
        session_key,
        content: parsed.text.to_string(),
        attachments: Vec::new(),
        channel: "telegram".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}
