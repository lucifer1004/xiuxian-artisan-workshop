use crate::llm::error::{LlmError, LlmResult};
use std::time::Duration;

pub fn validate_ocr_http_base_url(url: &str) -> Result<String, String> {
    if url.is_empty() {
        return Err("OCR client URL cannot be empty".to_string());
    }
    let trimmed = url.trim().trim_end_matches('/');
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("OCR client URL must start with http:// or https://".to_string());
    }
    Ok(trimmed.to_string())
}

pub async fn infer_remote_deepseek_ocr_from_bytes(
    base_url: &str,
    image_bytes: &[u8],
    media_type: &str,
) -> LlmResult<Option<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| internal_error(format!("failed to build HTTP client: {e}")))?;

    let url = format!("{base_url}/v1/vision/ocr");

    // Simple implementation for now
    let response = client
        .post(&url)
        .header("Content-Type", media_type)
        .body(image_bytes.to_vec())
        .send()
        .await
        .map_err(|e| internal_error(format!("OCR remote request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(internal_error(format!(
            "OCR remote error ({status}): {body}"
        )));
    }

    let markdown = response
        .text()
        .await
        .map_err(|e| internal_error(format!("failed to read OCR response: {e}")))?;

    if markdown.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(markdown))
    }
}

pub fn prewarm_remote_deepseek_ocr(base_url: &str) {
    tracing::info!(base_url, "Remote DeepSeek OCR prewarm requested (noop)");
}

fn internal_error(message: String) -> LlmError {
    LlmError::Internal { message }
}
