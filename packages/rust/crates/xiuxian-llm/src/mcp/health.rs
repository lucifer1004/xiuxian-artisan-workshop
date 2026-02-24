//! MCP health probing helpers.

use std::sync::OnceLock;
use std::time::Duration;

const DEFAULT_HEALTH_PROBE_TIMEOUT_MS: u64 = 1_500;

static HEALTH_PROBE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// MCP server health probe result.
#[derive(Debug, Clone)]
pub struct HealthProbeStatus {
    /// Compact summary for logs.
    pub summary: String,
    /// Parsed `ready` field when response body is structured JSON.
    pub ready: Option<bool>,
    /// Parsed `initializing` field when response body is structured JSON.
    pub initializing: Option<bool>,
    /// Whether both ready-state fields were found.
    pub has_structured_ready_state: bool,
    /// HTTP status code when request succeeded.
    pub status_code: Option<u16>,
    /// Request timed out.
    pub timed_out: bool,
    /// Non-timeout transport error.
    pub transport_error: bool,
}

/// Probe MCP `/health` and return a compact summary string.
pub async fn probe_health_summary(url: &str) -> String {
    probe_health_status(url).await.summary
}

/// Probe MCP `/health` and return structured status fields.
pub async fn probe_health_status(url: &str) -> HealthProbeStatus {
    let Some(health_url) = derive_health_url(url) else {
        return HealthProbeStatus {
            summary: "health_probe_skipped(invalid_url)".to_string(),
            ready: None,
            initializing: None,
            has_structured_ready_state: false,
            status_code: None,
            timed_out: false,
            transport_error: false,
        };
    };
    let client = match health_probe_client() {
        Ok(client) => client,
        Err(summary) => {
            return HealthProbeStatus {
                summary,
                ready: None,
                initializing: None,
                has_structured_ready_state: false,
                status_code: None,
                timed_out: false,
                transport_error: false,
            };
        }
    };
    match client.get(&health_url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&body) {
                let ready = payload
                    .get("ready")
                    .and_then(serde_json::Value::as_bool)
                    .map_or_else(|| "unknown".to_string(), |value| value.to_string());
                let initializing = payload
                    .get("initializing")
                    .and_then(serde_json::Value::as_bool)
                    .map_or_else(|| "unknown".to_string(), |value| value.to_string());
                let active_sessions = payload
                    .get("active_sessions")
                    .and_then(serde_json::Value::as_u64)
                    .map_or_else(|| "unknown".to_string(), |value| value.to_string());
                let parsed_ready = payload.get("ready").and_then(serde_json::Value::as_bool);
                let parsed_initializing = payload
                    .get("initializing")
                    .and_then(serde_json::Value::as_bool);
                return HealthProbeStatus {
                    summary: format!(
                        "health_status={status},ready={ready},initializing={initializing},active_sessions={active_sessions}"
                    ),
                    ready: parsed_ready,
                    initializing: parsed_initializing,
                    has_structured_ready_state: parsed_ready.is_some()
                        && parsed_initializing.is_some(),
                    status_code: Some(status),
                    timed_out: false,
                    transport_error: false,
                };
            }
            HealthProbeStatus {
                summary: format!("health_status={status}"),
                ready: None,
                initializing: None,
                has_structured_ready_state: false,
                status_code: Some(status),
                timed_out: false,
                transport_error: false,
            }
        }
        Err(error) => {
            if error.is_timeout() {
                HealthProbeStatus {
                    summary: "health_timeout".to_string(),
                    ready: None,
                    initializing: None,
                    has_structured_ready_state: false,
                    status_code: None,
                    timed_out: true,
                    transport_error: false,
                }
            } else {
                HealthProbeStatus {
                    summary: format!("health_error({error})"),
                    ready: None,
                    initializing: None,
                    has_structured_ready_state: false,
                    status_code: None,
                    timed_out: false,
                    transport_error: true,
                }
            }
        }
    }
}

fn health_probe_client() -> std::result::Result<&'static reqwest::Client, String> {
    if let Some(client) = HEALTH_PROBE_CLIENT.get() {
        return Ok(client);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(DEFAULT_HEALTH_PROBE_TIMEOUT_MS))
        .build()
        .map_err(|error| format!("health_probe_build_failed({error})"))?;
    let _ = HEALTH_PROBE_CLIENT.set(client);
    match HEALTH_PROBE_CLIENT.get() {
        Some(client) => Ok(client),
        None => Err("health_probe_build_failed(once_lock_not_initialized)".to_string()),
    }
}

/// Derive `/health` endpoint from streamable MCP endpoint.
#[must_use]
pub fn derive_health_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_trailing = trimmed.trim_end_matches('/');
    if let Some(base) = without_trailing.strip_suffix("/sse") {
        return Some(format!("{base}/health"));
    }
    if let Some(base) = without_trailing.strip_suffix("/messages") {
        return Some(format!("{base}/health"));
    }
    if let Some(base) = without_trailing.strip_suffix("/mcp") {
        return Some(format!("{base}/health"));
    }
    Some(format!("{without_trailing}/health"))
}
