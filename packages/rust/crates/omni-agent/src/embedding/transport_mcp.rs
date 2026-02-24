use std::time::Instant;

use reqwest::Client;

use super::types::McpEmbedResult;

pub(crate) async fn embed_mcp(
    client: &Client,
    mcp_url: Option<&str>,
    texts: &[String],
) -> Option<Vec<Vec<f32>>> {
    let url = mcp_url?;
    if texts.is_empty() {
        return Some(vec![]);
    }
    let started = Instant::now();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "mcp-embed",
        "method": "tools/call",
        "params": {
            "name": "embedding.embed_texts",
            "arguments": { "texts": texts }
        }
    });
    let resp = match client.post(url).json(&body).send().await {
        Ok(resp) => resp,
        Err(error) => {
            tracing::debug!(
                event = "agent.embedding.mcp.request_failed",
                url,
                elapsed_ms = started.elapsed().as_millis(),
                error = %error,
                "embedding mcp request failed"
            );
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::debug!(
            event = "agent.embedding.mcp.non_success_status",
            status = %resp.status(),
            elapsed_ms = started.elapsed().as_millis(),
            "embedding mcp returned non-success status"
        );
        return None;
    }
    let result: serde_json::Value = match resp.json().await {
        Ok(result) => result,
        Err(error) => {
            tracing::debug!(
                event = "agent.embedding.mcp.decode_failed",
                elapsed_ms = started.elapsed().as_millis(),
                error = %error,
                "embedding mcp response decode failed"
            );
            return None;
        }
    };
    let content = result.get("result")?.get("content")?.as_array()?;
    let text = content.first()?.get("text")?.as_str()?;
    let data: McpEmbedResult = match serde_json::from_str(text) {
        Ok(data) => data,
        Err(error) => {
            tracing::debug!(
                event = "agent.embedding.mcp.payload_parse_failed",
                elapsed_ms = started.elapsed().as_millis(),
                error = %error,
                "embedding mcp payload parse failed"
            );
            return None;
        }
    };
    if data.success {
        tracing::debug!(
            event = "agent.embedding.mcp.completed",
            elapsed_ms = started.elapsed().as_millis(),
            success = true,
            vector_count = data.vectors.len(),
            "embedding mcp path completed"
        );
        Some(data.vectors)
    } else {
        tracing::debug!(
            event = "agent.embedding.mcp.completed",
            elapsed_ms = started.elapsed().as_millis(),
            success = false,
            vector_count = data.vectors.len(),
            "embedding mcp path completed without success"
        );
        None
    }
}
