use crate::channels::traits::{ChannelAttachment, ChannelMessage};

use super::TelegramChannel;

const TELEGRAM_PHOTO_PLACEHOLDER: &str = "[telegram-photo]";
const TELEGRAM_IMAGE_PLACEHOLDER: &str = "[telegram-image]";

#[derive(Debug, Clone, Copy)]
struct PreferredImageCandidate<'a> {
    file_id: &'a str,
    file_size: u64,
    width: u32,
    height: u32,
    variant_count: usize,
    source: &'static str,
}

impl TelegramChannel {
    /// Enriches an inbound channel message with structured media attachments.
    ///
    /// This keeps image payloads as structured attachments so downstream LLM
    /// request builders can construct multimodal content instead of flattened
    /// marker-only text.
    pub async fn enrich_inbound_message_with_media_attachments(
        &self,
        update: &serde_json::Value,
        message: &mut ChannelMessage,
    ) {
        message.content = strip_image_placeholders(message.content.as_str());

        let Some(candidate) = extract_preferred_image_candidate(update) else {
            return;
        };
        tracing::info!(
            event = "agent.channel.telegram.inbound_media.image_selected",
            source = candidate.source,
            file_id = candidate.file_id,
            file_size = candidate.file_size,
            width = candidate.width,
            height = candidate.height,
            variants = candidate.variant_count,
            "Telegram inbound image attachment selected for multimodal OCR"
        );

        let Some(file_url) = self.resolve_file_url(candidate.file_id).await else {
            return;
        };

        if !message.attachments.iter().any(|attachment| {
            matches!(
                attachment,
                ChannelAttachment::ImageUrl { url } if url == file_url.as_str()
            )
        }) {
            message
                .attachments
                .push(ChannelAttachment::ImageUrl { url: file_url });
        }
    }

    async fn resolve_file_url(&self, file_id: &str) -> Option<String> {
        let request = serde_json::json!({ "file_id": file_id });
        let response = self
            .client
            .post(self.api_url("getFile"))
            .json(&request)
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            tracing::debug!(
                status = %response.status(),
                file_id,
                "telegram getFile request failed"
            );
            return None;
        }

        let payload = response.json::<serde_json::Value>().await.ok()?;
        if !payload
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
        {
            tracing::debug!(file_id, "telegram getFile API responded with ok=false");
            return None;
        }

        let file_path = payload
            .get("result")
            .and_then(|result| result.get("file_path"))
            .and_then(serde_json::Value::as_str)?;
        let base = self.api_base_url.trim_end_matches('/');
        Some(format!(
            "{base}/file/bot{}/{}",
            self.bot_token,
            file_path.trim_start_matches('/')
        ))
    }
}

fn extract_preferred_image_candidate(
    update: &serde_json::Value,
) -> Option<PreferredImageCandidate<'_>> {
    let message = update.get("message")?;
    let photo_candidate = message
        .get("photo")
        .and_then(serde_json::Value::as_array)
        .and_then(|photos| {
            photos
                .iter()
                .filter_map(|item| {
                    let file_id = item.get("file_id").and_then(serde_json::Value::as_str)?;
                    let file_size = item
                        .get("file_size")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or_default();
                    let width = item
                        .get("width")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(to_u32)
                        .unwrap_or_default();
                    let height = item
                        .get("height")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(to_u32)
                        .unwrap_or_default();
                    Some(PreferredImageCandidate {
                        file_id,
                        file_size,
                        width,
                        height,
                        variant_count: photos.len(),
                        source: "photo",
                    })
                })
                .max_by_key(|candidate| {
                    (
                        candidate.file_size,
                        u64::from(candidate.width) * u64::from(candidate.height),
                    )
                })
        });
    if photo_candidate.is_some() {
        return photo_candidate;
    }

    message.get("document").and_then(|document| {
        let mime = document
            .get("mime_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !mime.starts_with("image/") {
            return None;
        }
        let file_id = document
            .get("file_id")
            .and_then(serde_json::Value::as_str)?;
        let file_size = document
            .get("file_size")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        Some(PreferredImageCandidate {
            file_id,
            file_size,
            width: 0,
            height: 0,
            variant_count: 1,
            source: "document",
        })
    })
}

fn to_u32(value: u64) -> Option<u32> {
    u32::try_from(value).ok()
}

fn strip_image_placeholders(content: &str) -> String {
    content
        .replace(TELEGRAM_PHOTO_PLACEHOLDER, "")
        .replace(TELEGRAM_IMAGE_PLACEHOLDER, "")
        .trim()
        .to_string()
}
