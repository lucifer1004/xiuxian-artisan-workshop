use std::time::Duration;

use pulldown_cmark::{Event, Options, Parser, Tag};

use super::TelegramChannel;
use super::chunking::{decorate_chunk_for_telegram, split_message_for_telegram};
use super::constants::{TELEGRAM_MAX_AUTO_TEXT_CHUNKS, TELEGRAM_MAX_MESSAGE_LENGTH};
use super::identity::parse_recipient_target;
use super::markdown::{markdown_to_telegram_html, markdown_to_telegram_markdown_v2};
use super::media::{parse_attachment_markers, parse_path_only_attachment};
use super::outbound_text::normalize_telegram_outbound_text;
use super::send_types::PreparedCaption;

struct PreparedChunk {
    plain_text: String,
    markdown_v2_text: String,
    markdown_chars: usize,
    html_text: String,
    html_chars: usize,
}

fn should_prefer_html_chunk(plain_text: &str, markdown_chars: usize, html_chars: usize) -> bool {
    markdown_chars.saturating_sub(html_chars) >= 256 || markdown_ast_contains_images(plain_text)
}

fn markdown_ast_contains_images(markdown: &str) -> bool {
    Parser::new_ext(markdown, Options::all())
        .any(|event| matches!(event, Event::Start(Tag::Image { .. })))
}

impl TelegramChannel {
    pub(super) async fn send_text(&self, message: &str, recipient: &str) -> anyhow::Result<()> {
        let (chat_id, thread_id) = parse_recipient_target(recipient);
        let normalized_message = normalize_telegram_outbound_text(message);

        let (text_without_markers, attachments, has_invalid_attachment_marker) =
            parse_attachment_markers(&normalized_message);
        if !attachments.is_empty() {
            let first_attachment_caption =
                Self::select_first_attachment_caption(&text_without_markers, &attachments)
                    .map(|caption| PreparedCaption::from_plain(caption.as_str()));

            if first_attachment_caption.is_none() && !text_without_markers.is_empty() {
                self.send_text_chunks(&text_without_markers, chat_id, thread_id, false)
                    .await?;
            }
            self.send_attachments(
                chat_id,
                thread_id,
                &attachments,
                first_attachment_caption.as_ref(),
            )
            .await?;
            return Ok(());
        }

        if has_invalid_attachment_marker {
            return self
                .send_text_chunks(&text_without_markers, chat_id, thread_id, true)
                .await;
        }

        if let Some(attachment) = parse_path_only_attachment(&normalized_message) {
            self.send_attachments(chat_id, thread_id, &[attachment], None)
                .await?;
            return Ok(());
        }

        self.send_text_chunks(&normalized_message, chat_id, thread_id, false)
            .await
    }

    #[allow(clippy::too_many_lines)]
    async fn send_text_chunks(
        &self,
        message: &str,
        chat_id: &str,
        thread_id: Option<&str>,
        force_plain: bool,
    ) -> anyhow::Result<()> {
        let mut chunks = split_message_for_telegram(message);
        let truncated = if chunks.len() > TELEGRAM_MAX_AUTO_TEXT_CHUNKS {
            let total_chunks = chunks.len();
            chunks.truncate(TELEGRAM_MAX_AUTO_TEXT_CHUNKS);
            let kept_chunks = chunks.len();
            tracing::warn!(
                total_chunks,
                kept_chunks,
                "Telegram message exceeds auto-chunk guard; truncating output to prevent flood"
            );
            Some((total_chunks, kept_chunks))
        } else {
            None
        };

        let prepared_chunks: Vec<PreparedChunk> = chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let plain_text = decorate_chunk_for_telegram(chunk, index, chunks.len());
                let markdown_v2_text = markdown_to_telegram_markdown_v2(&plain_text);
                let html_text = markdown_to_telegram_html(&plain_text);
                let markdown_chars = markdown_v2_text.chars().count();
                let html_chars = html_text.chars().count();
                PreparedChunk {
                    plain_text,
                    markdown_v2_text,
                    markdown_chars,
                    html_text,
                    html_chars,
                }
            })
            .collect();

        let markdown_overflow_chunks = prepared_chunks
            .iter()
            .filter(|chunk| chunk.markdown_chars > TELEGRAM_MAX_MESSAGE_LENGTH)
            .count();
        let html_overflow_chunks = prepared_chunks
            .iter()
            .filter(|chunk| {
                chunk.markdown_chars > TELEGRAM_MAX_MESSAGE_LENGTH
                    && chunk.html_chars > TELEGRAM_MAX_MESSAGE_LENGTH
            })
            .count();
        let prefer_html_chunks = prepared_chunks
            .iter()
            .filter(|chunk| {
                should_prefer_html_chunk(&chunk.plain_text, chunk.markdown_chars, chunk.html_chars)
            })
            .count();

        if markdown_overflow_chunks > 0 {
            tracing::warn!(
                chunks = prepared_chunks.len(),
                markdown_overflow_chunks,
                html_overflow_chunks,
                prefer_html_chunks,
                "Telegram MarkdownV2 payload exceeds limit for some chunks; using per-chunk fallback"
            );
        }

        let total_chunks = prepared_chunks.len();
        for (index, chunk) in prepared_chunks.into_iter().enumerate() {
            let prefer_html_for_chunk =
                should_prefer_html_chunk(&chunk.plain_text, chunk.markdown_chars, chunk.html_chars);
            if force_plain {
                self.send_message_with_mode(chat_id, thread_id, &chunk.plain_text, None)
                    .await
                    .map_err(|plain_error| {
                        anyhow::anyhow!(
                            "Telegram sendMessage failed (forced plain mode: {plain_error})"
                        )
                    })?;
            } else if chunk.markdown_chars > TELEGRAM_MAX_MESSAGE_LENGTH {
                if chunk.html_chars <= TELEGRAM_MAX_MESSAGE_LENGTH {
                    let html_result = self
                        .send_message_with_mode(chat_id, thread_id, &chunk.html_text, Some("HTML"))
                        .await;
                    match html_result {
                        Ok(()) => {}
                        Err(html_error) if html_error.should_retry_without_parse_mode() => {
                            tracing::warn!(
                                error = %html_error,
                                "Telegram HTML send failed with parse-mode error for oversized markdown chunk; retrying without parse_mode"
                            );
                            self.send_message_with_mode(chat_id, thread_id, &chunk.plain_text, None)
                                .await
                                .map_err(|plain_error| {
                                    anyhow::anyhow!(
                                        "Telegram sendMessage failed (markdown exceeded limit; html fallback failed: {html_error}; plain fallback: {plain_error})"
                                    )
                                })?;
                        }
                        Err(error) => {
                            anyhow::bail!(
                                "Telegram sendMessage failed (markdown exceeded limit; html fallback failed: {error})"
                            );
                        }
                    }
                } else {
                    self.send_message_with_mode(chat_id, thread_id, &chunk.plain_text, None)
                        .await
                        .map_err(|plain_error| {
                            anyhow::anyhow!(
                                "Telegram sendMessage failed (markdown/html exceeded size limits; plain fallback: {plain_error})"
                            )
                        })?;
                }
            } else if prefer_html_for_chunk && chunk.html_chars <= TELEGRAM_MAX_MESSAGE_LENGTH {
                let html_result = self
                    .send_message_with_mode(chat_id, thread_id, &chunk.html_text, Some("HTML"))
                    .await;
                match html_result {
                    Ok(()) => {}
                    Err(html_error) if html_error.should_retry_without_parse_mode() => {
                        tracing::warn!(
                            error = %html_error,
                            "Telegram preferred HTML send failed with parse-mode error; retrying without parse_mode"
                        );
                        self.send_message_with_mode(chat_id, thread_id, &chunk.plain_text, None)
                            .await
                            .map_err(|plain_error| {
                                anyhow::anyhow!(
                                    "Telegram sendMessage failed (preferred html fallback failed: {html_error}; plain fallback: {plain_error})"
                                )
                            })?;
                    }
                    Err(error) => {
                        anyhow::bail!(
                            "Telegram sendMessage failed (preferred html send failed: {error})"
                        );
                    }
                }
            } else {
                let send_result = self
                    .send_message_with_mode(
                        chat_id,
                        thread_id,
                        &chunk.markdown_v2_text,
                        Some("MarkdownV2"),
                    )
                    .await;

                match send_result {
                    Ok(()) => {}
                    Err(markdown_error) if markdown_error.should_retry_without_parse_mode() => {
                        tracing::warn!(
                            error = %markdown_error,
                            "Telegram MarkdownV2 send failed with parse-mode error; retrying with HTML parse mode"
                        );
                        if chunk.html_chars <= TELEGRAM_MAX_MESSAGE_LENGTH {
                            let html_result = self
                                .send_message_with_mode(
                                    chat_id,
                                    thread_id,
                                    &chunk.html_text,
                                    Some("HTML"),
                                )
                                .await;
                            match html_result {
                                Ok(()) => {}
                                Err(html_error) if html_error.should_retry_without_parse_mode() => {
                                    tracing::warn!(
                                        error = %html_error,
                                        "Telegram HTML send failed with parse-mode error; retrying without parse_mode"
                                    );
                                    self.send_message_with_mode(
                                        chat_id,
                                        thread_id,
                                        &chunk.plain_text,
                                        None,
                                    )
                                        .await
                                        .map_err(|plain_error| {
                                            anyhow::anyhow!(
                                                "Telegram sendMessage failed (markdown request failed: {markdown_error}; html fallback failed: {html_error}; plain fallback: {plain_error})"
                                            )
                                        })?;
                                }
                                Err(error) => {
                                    anyhow::bail!(
                                        "Telegram sendMessage failed (markdown request failed: {markdown_error}; html fallback failed: {error})"
                                    );
                                }
                            }
                        } else {
                            tracing::warn!(
                                html_chars = chunk.html_chars,
                                "Telegram HTML fallback chunk exceeds message limit; sending plain text"
                            );
                            self.send_message_with_mode(chat_id, thread_id, &chunk.plain_text, None)
                                .await
                                .map_err(|plain_error| {
                                    anyhow::anyhow!(
                                        "Telegram sendMessage failed (markdown request failed: {markdown_error}; plain fallback: {plain_error})"
                                    )
                                })?;
                        }
                    }
                    Err(error) => {
                        anyhow::bail!("Telegram sendMessage failed: {error}");
                    }
                }
            }

            debug_assert!(
                chunk.plain_text.chars().count() <= TELEGRAM_MAX_MESSAGE_LENGTH,
                "chunk {} exceeds limit: {} > {}",
                index,
                chunk.plain_text.chars().count(),
                TELEGRAM_MAX_MESSAGE_LENGTH
            );

            if index < total_chunks - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        if let Some((total_chunks, kept_chunks)) = truncated {
            let notice = format!(
                "Output truncated after {kept_chunks} of {total_chunks} chunks to prevent flood. Narrow the query or request paginated output."
            );
            self.send_message_with_mode(chat_id, thread_id, &notice, None)
                .await
                .map_err(|error| {
                    anyhow::anyhow!("Telegram sendMessage failed (truncation notice): {error}")
                })?;
        }
        Ok(())
    }
}
