use std::time::{Duration, Instant};

use reqwest::Client;
use serde::Deserialize;

const OPENAI_HTTP_RETRY_DELAY_MS: u64 = 40;
const OPENAI_HTTP_MAX_ATTEMPTS: usize = 2;

#[derive(Deserialize)]
struct OpenAiEmbeddingsResponse {
    #[serde(default)]
    data: Vec<OpenAiEmbeddingItem>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingItem {
    embedding: Vec<f32>,
}

/// Normalize upstream base URL to a concrete OpenAI-compatible embeddings endpoint.
#[must_use]
pub fn normalize_openai_embeddings_url(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with("/v1") {
        return Some(format!("{trimmed}/embeddings"));
    }
    Some(format!("{trimmed}/v1/embeddings"))
}

/// Embed a batch through an OpenAI-compatible `/v1/embeddings` API.
///
/// Returns `None` on network/HTTP/parse failures so callers can apply fallback policy.
pub async fn embed_openai_compatible(
    client: &Client,
    base_url: &str,
    texts: &[String],
    model: Option<&str>,
) -> Option<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Some(vec![]);
    }
    let url = normalize_openai_embeddings_url(base_url)?;
    let started = Instant::now();
    let mut body = serde_json::json!({ "input": texts });
    if let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) {
        body["model"] = serde_json::Value::String(model.to_string());
    }
    for attempt in 1..=OPENAI_HTTP_MAX_ATTEMPTS {
        let resp = match client.post(&url).json(&body).send().await {
            Ok(resp) => resp,
            Err(error) => {
                let should_retry =
                    attempt < OPENAI_HTTP_MAX_ATTEMPTS && should_retry_http_request_error(&error);
                if should_retry {
                    tracing::debug!(
                        event = "xiuxian.llm.embedding.openai_http.request_retry",
                        url,
                        attempt,
                        max_attempts = OPENAI_HTTP_MAX_ATTEMPTS,
                        elapsed_ms = started.elapsed().as_millis(),
                        error = %error,
                        "embedding openai-compatible request failed; retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(OPENAI_HTTP_RETRY_DELAY_MS)).await;
                    continue;
                }
                tracing::debug!(
                    event = "xiuxian.llm.embedding.openai_http.request_failed",
                    url,
                    attempt,
                    max_attempts = OPENAI_HTTP_MAX_ATTEMPTS,
                    elapsed_ms = started.elapsed().as_millis(),
                    error = %error,
                    "embedding openai-compatible request failed"
                );
                return None;
            }
        };
        if !resp.status().is_success() {
            let should_retry =
                attempt < OPENAI_HTTP_MAX_ATTEMPTS && resp.status().is_server_error();
            if should_retry {
                tracing::debug!(
                    event = "xiuxian.llm.embedding.openai_http.retry_on_server_error",
                    status = %resp.status(),
                    attempt,
                    max_attempts = OPENAI_HTTP_MAX_ATTEMPTS,
                    elapsed_ms = started.elapsed().as_millis(),
                    "embedding openai-compatible returned server error; retrying"
                );
                tokio::time::sleep(Duration::from_millis(OPENAI_HTTP_RETRY_DELAY_MS)).await;
                continue;
            }
            tracing::debug!(
                event = "xiuxian.llm.embedding.openai_http.non_success_status",
                status = %resp.status(),
                attempt,
                max_attempts = OPENAI_HTTP_MAX_ATTEMPTS,
                elapsed_ms = started.elapsed().as_millis(),
                "embedding openai-compatible returned non-success status"
            );
            return None;
        }

        let data: OpenAiEmbeddingsResponse = match resp.json().await {
            Ok(data) => data,
            Err(error) => {
                tracing::debug!(
                    event = "xiuxian.llm.embedding.openai_http.decode_failed",
                    elapsed_ms = started.elapsed().as_millis(),
                    attempt,
                    max_attempts = OPENAI_HTTP_MAX_ATTEMPTS,
                    error = %error,
                    "embedding openai-compatible response decode failed"
                );
                return None;
            }
        };
        let vectors = data
            .data
            .into_iter()
            .map(|item| item.embedding)
            .collect::<Vec<_>>();
        tracing::debug!(
            event = "xiuxian.llm.embedding.openai_http.completed",
            elapsed_ms = started.elapsed().as_millis(),
            attempt,
            max_attempts = OPENAI_HTTP_MAX_ATTEMPTS,
            success = true,
            vector_count = vectors.len(),
            "embedding openai-compatible path completed"
        );
        return Some(vectors);
    }
    None
}

fn should_retry_http_request_error(error: &reqwest::Error) -> bool {
    error.is_connect()
        || error.is_timeout()
        || error.to_string().contains("error sending request for url")
}
