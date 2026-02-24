use std::sync::Arc;

use axum::http::StatusCode;

use crate::config::load_runtime_settings;
use crate::embedding::EmbeddingClient;

use super::types::GatewayEmbeddingRuntime;

const DEFAULT_EMBED_TIMEOUT_SECS: u64 = 15;
const DEFAULT_EMBED_UPSTREAM_BASE_URL: &str = "http://127.0.0.1:11434";

pub(super) fn trim_non_empty(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn resolve_embed_model(
    requested_model: Option<&str>,
    default_model: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    let configured = trim_non_empty(default_model);
    let requested = trim_non_empty(requested_model);

    if let Some(configured_model) = configured {
        if let Some(requested_model) = requested.as_deref()
            && requested_model != configured_model
        {
            tracing::debug!(
                event = "gateway.embedding.model_override_ignored",
                requested_model,
                configured_model,
                "ignoring request model override; using configured embedding model"
            );
        }
        return Ok(configured_model);
    }

    requested.ok_or((
        StatusCode::BAD_REQUEST,
        "embedding model must be provided (request.model or embedding.model)".to_string(),
    ))
}

pub(super) fn build_embedding_runtime() -> GatewayEmbeddingRuntime {
    let runtime_settings = load_runtime_settings();
    let backend_hint = trim_non_empty(runtime_settings.memory.embedding_backend.as_deref())
        .or_else(|| trim_non_empty(runtime_settings.embedding.backend.as_deref()));
    let base_url = trim_non_empty(runtime_settings.memory.embedding_base_url.as_deref())
        .or_else(|| trim_non_empty(runtime_settings.embedding.litellm_api_base.as_deref()))
        .or_else(|| trim_non_empty(runtime_settings.embedding.client_url.as_deref()))
        .unwrap_or_else(|| DEFAULT_EMBED_UPSTREAM_BASE_URL.to_string());
    let timeout_secs = runtime_settings
        .embedding
        .timeout_secs
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_EMBED_TIMEOUT_SECS);
    let batch_max_size = runtime_settings
        .embedding
        .batch_max_size
        .filter(|value| *value > 0);
    let batch_max_concurrency = runtime_settings
        .embedding
        .batch_max_concurrency
        .filter(|value| *value > 0);
    let default_model = trim_non_empty(runtime_settings.memory.embedding_model.as_deref())
        .or_else(|| trim_non_empty(runtime_settings.embedding.litellm_model.as_deref()))
        .or_else(|| trim_non_empty(runtime_settings.embedding.model.as_deref()));

    tracing::info!(
        event = "gateway.embedding.runtime.initialized",
        backend = backend_hint.as_deref().unwrap_or("default"),
        base_url = %base_url,
        timeout_secs,
        has_default_model = default_model.is_some(),
        batch_max_size = ?batch_max_size,
        batch_max_concurrency = ?batch_max_concurrency,
        "gateway embedding runtime initialized"
    );

    let client = EmbeddingClient::new_with_backend_and_tuning(
        &base_url,
        timeout_secs,
        backend_hint.as_deref(),
        batch_max_size,
        batch_max_concurrency,
    );

    GatewayEmbeddingRuntime {
        client: Arc::new(client),
        default_model,
    }
}

#[cfg(test)]
#[path = "../../../tests/gateway/http/runtime.rs"]
mod tests;
