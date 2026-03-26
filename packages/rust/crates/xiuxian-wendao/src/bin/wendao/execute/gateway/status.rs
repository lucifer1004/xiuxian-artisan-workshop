//! Gateway status endpoints.

use std::sync::Arc;

use axum::{Json, extract::State};
use serde_json::json;

use crate::execute::gateway::shared::AppState;
use xiuxian_wendao::LinkGraphIndex;

/// Stats endpoint.
pub(crate) async fn stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match &state.index {
        Some(index) => {
            let payload = LinkGraphIndex::stats(index.as_ref());
            Json(
                serde_json::to_value(payload)
                    .unwrap_or_else(|_| json!({"error": "serialization failed"})),
            )
        }
        None => Json(json!({"error": "no index loaded"})),
    }
}

/// Notification service status endpoint.
pub(crate) async fn notify_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let has_signal_channel = state.signal_tx.is_some();
    let webhook_url =
        std::env::var("WENDAO_WEBHOOK_URL").unwrap_or_else(|_| "not configured".to_string());

    Json(json!({
        "notification_worker": if has_signal_channel { "active" } else { "inactive" },
        "webhook_configured": !webhook_url.is_empty(),
        "webhook_url": if webhook_url.is_empty() { serde_json::Value::Null } else { json!(webhook_url) }
    }))
}
