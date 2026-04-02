//! HTTP gateway helpers exposed for integration tests.

use std::sync::Arc;

use axum::http::StatusCode;

use crate::gateway::http::{handlers, llm_proxy, runtime};
use crate::{EmbeddingClient, RuntimeSettings};

/// Test-facing gateway embedding runtime descriptor.
#[derive(Clone)]
pub struct GatewayEmbeddingRuntimeHandle {
    pub client: Arc<EmbeddingClient>,
    pub default_model: Option<String>,
}

/// Resolve effective embedding model for one request.
///
/// # Errors
///
/// Returns `BAD_REQUEST` when both request and configured defaults are absent.
pub fn resolve_embed_model(
    requested_model: Option<&str>,
    default_model: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    runtime::resolve_embed_model(requested_model, default_model)
}

/// Resolve embedding upstream base URL from runtime settings.
#[must_use]
pub fn resolve_embed_base_url(
    runtime_settings: &RuntimeSettings,
    backend_hint: Option<&str>,
) -> String {
    runtime::resolve_embed_base_url(runtime_settings, backend_hint)
}

/// Resolve embedding base URL with optional explicit override.
#[must_use]
pub fn resolve_runtime_embed_base_url(
    runtime_settings: &RuntimeSettings,
    backend_hint: Option<&str>,
    base_url_override: Option<&str>,
) -> String {
    runtime::resolve_runtime_embed_base_url(runtime_settings, backend_hint, base_url_override)
}

/// Build gateway embedding runtime directly from explicit runtime settings.
#[must_use]
pub fn build_embedding_runtime_for_settings(
    runtime_settings: &RuntimeSettings,
) -> GatewayEmbeddingRuntimeHandle {
    let runtime = runtime::build_embedding_runtime_for_settings(runtime_settings);
    GatewayEmbeddingRuntimeHandle {
        client: runtime.client,
        default_model: runtime.default_model,
    }
}

/// Deterministic hash fallback used by embedding gateway tests.
#[must_use]
pub fn fallback_hash_embed_batch(inputs: &[String], dimension: usize) -> Vec<Vec<f32>> {
    handlers::fallback_hash_embed_batch(inputs, dimension)
}

/// Apply gateway embedding-memory guard with explicit env inputs for tests.
#[must_use]
pub fn apply_gateway_embedding_memory_guard_for_tests(
    runtime_settings: &RuntimeSettings,
    env_memory_backend: Option<&str>,
    env_embed_backend: Option<&str>,
    allow_inproc_embed_raw: Option<&str>,
) -> RuntimeSettings {
    runtime::apply_gateway_embedding_memory_guard_for_tests(
        runtime_settings,
        env_memory_backend,
        env_embed_backend,
        allow_inproc_embed_raw,
    )
}

/// Resolve LLM-proxy target base URL from override/default candidates.
#[must_use]
pub fn resolve_target_base_url(base_url_override: Option<&str>, resolved_base_url: &str) -> String {
    llm_proxy::resolve_target_base_url(base_url_override, resolved_base_url)
}

/// Resolve LLM-proxy API-key env name from override/default candidates.
#[must_use]
pub fn resolve_target_api_key_env(
    api_key_env_override: Option<&str>,
    resolved_key_env: &str,
) -> String {
    llm_proxy::resolve_target_api_key_env(api_key_env_override, resolved_key_env)
}

/// Read one API key from literal/env reference value.
#[must_use]
pub fn read_api_key(raw: &str) -> String {
    llm_proxy::read_api_key(raw)
}

/// Resolve request model with request/settings/xiuxian precedence.
#[must_use]
pub fn resolve_request_model(
    request_model: Option<&str>,
    inference_default_model: Option<&str>,
    xiuxian_default_model: Option<&str>,
) -> Option<String> {
    llm_proxy::resolve_request_model(
        request_model,
        inference_default_model,
        xiuxian_default_model,
    )
}
