use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;

use super::backend::{EmbeddingBackendMode, resolve_backend_settings};
use super::cache::EmbeddingCache;
use super::transport_http::embed_http;
#[cfg(feature = "agent-provider-litellm")]
use super::transport_litellm::embed_litellm;
use super::transport_mcp::embed_mcp;
use super::transport_openai::embed_openai_http;
#[cfg(feature = "agent-provider-litellm")]
use crate::config::load_runtime_settings;

const DEFAULT_EMBED_CACHE_TTL_SECS: u64 = 900;
const MAX_EMBED_CACHE_TTL_SECS: u64 = 86_400;
const DEFAULT_EMBED_CACHE_MAX_ENTRIES: usize = 4_096;
const MAX_EMBED_CACHE_MAX_ENTRIES: usize = 65_536;
const DEFAULT_EMBED_BATCH_MAX_SIZE: usize = 128;
const MAX_EMBED_BATCH_MAX_SIZE: usize = 8_192;
const DEFAULT_EMBED_BATCH_MAX_CONCURRENCY: usize = 1;
const MAX_EMBED_BATCH_MAX_CONCURRENCY: usize = 64;

/// Embedding client runtime.
pub struct EmbeddingClient {
    client: reqwest::Client,
    base_url: String,
    mcp_url: Option<String>,
    cache: EmbeddingCache,
    backend_mode: EmbeddingBackendMode,
    backend_source: &'static str,
    #[cfg(feature = "agent-provider-litellm")]
    timeout_secs: u64,
    max_in_flight: Option<usize>,
    in_flight_gate: Option<Arc<Semaphore>>,
    batch_max_size: usize,
    batch_max_concurrency: usize,
    default_model: Option<String>,
    #[cfg(feature = "agent-provider-litellm")]
    litellm_api_key: Option<String>,
}

#[derive(Clone)]
struct EmbeddingDispatchRuntime {
    client: reqwest::Client,
    base_url: String,
    mcp_url: Option<String>,
    backend_mode: EmbeddingBackendMode,
    backend_source: &'static str,
    #[cfg(feature = "agent-provider-litellm")]
    timeout_secs: u64,
    max_in_flight: Option<usize>,
    in_flight_gate: Option<Arc<Semaphore>>,
    #[cfg(feature = "agent-provider-litellm")]
    litellm_api_key: Option<String>,
}

impl EmbeddingClient {
    #[must_use]
    pub fn new(base_url: &str, timeout_secs: u64) -> Self {
        let mcp_url = std::env::var("OMNI_MCP_EMBED_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Self::new_with_mcp_url_and_backend(base_url, timeout_secs, mcp_url, None)
    }

    #[must_use]
    pub fn new_with_backend(base_url: &str, timeout_secs: u64, backend_hint: Option<&str>) -> Self {
        let mcp_url = std::env::var("OMNI_MCP_EMBED_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Self::new_with_mcp_url_and_backend(base_url, timeout_secs, mcp_url, backend_hint)
    }

    #[must_use]
    pub fn new_with_backend_and_tuning(
        base_url: &str,
        timeout_secs: u64,
        backend_hint: Option<&str>,
        batch_max_size_hint: Option<usize>,
        batch_max_concurrency_hint: Option<usize>,
    ) -> Self {
        let mcp_url = std::env::var("OMNI_MCP_EMBED_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Self::new_with_mcp_url_and_backend_and_tuning(
            base_url,
            timeout_secs,
            mcp_url,
            backend_hint,
            batch_max_size_hint,
            batch_max_concurrency_hint,
        )
    }

    #[must_use]
    pub fn new_with_mcp_url(base_url: &str, timeout_secs: u64, mcp_url: Option<String>) -> Self {
        Self::new_with_mcp_url_and_backend_and_tuning(
            base_url,
            timeout_secs,
            mcp_url,
            None,
            None,
            None,
        )
    }

    #[must_use]
    pub fn new_with_mcp_url_and_backend(
        base_url: &str,
        timeout_secs: u64,
        mcp_url: Option<String>,
        backend_hint: Option<&str>,
    ) -> Self {
        Self::new_with_mcp_url_and_backend_and_tuning(
            base_url,
            timeout_secs,
            mcp_url,
            backend_hint,
            None,
            None,
        )
    }

    #[must_use]
    pub fn new_with_mcp_url_and_backend_and_tuning(
        base_url: &str,
        timeout_secs: u64,
        mcp_url: Option<String>,
        backend_hint: Option<&str>,
        batch_max_size_hint: Option<usize>,
        batch_max_concurrency_hint: Option<usize>,
    ) -> Self {
        let backend_settings = resolve_backend_settings(timeout_secs, backend_hint);
        let cache_ttl_secs = parse_positive_env_u64(
            "OMNI_AGENT_EMBED_CACHE_TTL_SECS",
            DEFAULT_EMBED_CACHE_TTL_SECS,
            MAX_EMBED_CACHE_TTL_SECS,
        );
        let cache_max_entries = parse_positive_env_usize(
            "OMNI_AGENT_EMBED_CACHE_MAX_ENTRIES",
            DEFAULT_EMBED_CACHE_MAX_ENTRIES,
            MAX_EMBED_CACHE_MAX_ENTRIES,
        );
        let batch_max_size = batch_max_size_hint
            .filter(|value| *value > 0)
            .map(|value| value.min(MAX_EMBED_BATCH_MAX_SIZE))
            .map_or_else(
                || {
                    parse_positive_env_usize(
                        "OMNI_AGENT_EMBED_BATCH_MAX_SIZE",
                        DEFAULT_EMBED_BATCH_MAX_SIZE,
                        MAX_EMBED_BATCH_MAX_SIZE,
                    )
                },
                std::convert::identity,
            );
        let batch_max_concurrency = batch_max_concurrency_hint
            .filter(|value| *value > 0)
            .map(|value| value.min(MAX_EMBED_BATCH_MAX_CONCURRENCY))
            .map_or_else(
                || {
                    parse_positive_env_usize(
                        "OMNI_AGENT_EMBED_BATCH_MAX_CONCURRENCY",
                        DEFAULT_EMBED_BATCH_MAX_CONCURRENCY,
                        MAX_EMBED_BATCH_MAX_CONCURRENCY,
                    )
                },
                std::convert::identity,
            );
        let in_flight_gate = backend_settings
            .max_in_flight
            .map(|limit| Arc::new(Semaphore::new(limit)));
        #[cfg(feature = "agent-provider-litellm")]
        let litellm_api_key = resolve_litellm_embed_api_key();
        tracing::info!(
            embed_backend = backend_settings.mode.as_str(),
            embed_backend_source = backend_settings.source,
            embed_timeout_secs = backend_settings.timeout_secs,
            embed_max_in_flight = backend_settings.max_in_flight,
            embed_batch_max_size = batch_max_size,
            embed_batch_max_concurrency = batch_max_concurrency,
            has_default_model = backend_settings.default_model.is_some(),
            "embedding backend selected"
        );
        Self {
            client: build_http_client(backend_settings.timeout_secs),
            base_url: base_url.trim_end_matches('/').to_string(),
            mcp_url,
            cache: EmbeddingCache::new(Duration::from_secs(cache_ttl_secs), cache_max_entries),
            backend_mode: backend_settings.mode,
            backend_source: backend_settings.source,
            #[cfg(feature = "agent-provider-litellm")]
            timeout_secs: backend_settings.timeout_secs,
            max_in_flight: backend_settings.max_in_flight,
            in_flight_gate,
            batch_max_size,
            batch_max_concurrency,
            default_model: backend_settings.default_model,
            #[cfg(feature = "agent-provider-litellm")]
            litellm_api_key,
        }
    }

    /// Embed texts with an optional embedding model hint.
    pub async fn embed_batch_with_model(
        &self,
        texts: &[String],
        model: Option<&str>,
    ) -> Option<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Some(vec![]);
        }
        let resolved_model = model
            .map(str::trim)
            .map(ToString::to_string)
            .filter(|value| !value.is_empty())
            .or_else(|| self.default_model.clone());
        let started = Instant::now();
        if let Some(cached) = self.cache.get_batch(texts, resolved_model.as_deref()).await {
            tracing::debug!(
                event = "agent.embedding.cache.hit",
                batch_size = texts.len(),
                elapsed_ms = started.elapsed().as_millis(),
                "embedding batch served from local cache"
            );
            return Some(cached);
        }
        let chunk_ranges = build_chunk_ranges(texts.len(), self.batch_max_size);
        let chunk_count = chunk_ranges.len();
        let effective_chunk_concurrency = self.batch_max_concurrency.max(1).min(chunk_count.max(1));
        tracing::debug!(
            event = "agent.embedding.batch.plan",
            backend = self.backend_mode.as_str(),
            backend_source = self.backend_source,
            batch_size = texts.len(),
            model = resolved_model.as_deref().unwrap_or(""),
            chunk_count,
            chunk_max_size = self.batch_max_size,
            chunk_concurrency = effective_chunk_concurrency,
            max_in_flight = self.max_in_flight,
            "embedding batch execution plan prepared"
        );

        let runtime = self.dispatch_runtime();
        let result = self
            .dispatch_embeddings_for_ranges(
                &runtime,
                texts,
                resolved_model.as_deref(),
                &chunk_ranges,
                effective_chunk_concurrency,
            )
            .await;

        if let Some(vectors) = result.as_ref() {
            self.cache
                .put_batch(texts, vectors, resolved_model.as_deref())
                .await;
        }
        tracing::debug!(
            event = "agent.embedding.batch.completed",
            backend = self.backend_mode.as_str(),
            backend_source = self.backend_source,
            success = result.is_some(),
            elapsed_ms = started.elapsed().as_millis(),
            "embedding batch completed"
        );
        result
    }

    fn dispatch_runtime(&self) -> EmbeddingDispatchRuntime {
        EmbeddingDispatchRuntime {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            mcp_url: self.mcp_url.clone(),
            backend_mode: self.backend_mode,
            backend_source: self.backend_source,
            #[cfg(feature = "agent-provider-litellm")]
            timeout_secs: self.timeout_secs,
            max_in_flight: self.max_in_flight,
            in_flight_gate: self.in_flight_gate.clone(),
            #[cfg(feature = "agent-provider-litellm")]
            litellm_api_key: self.litellm_api_key.clone(),
        }
    }

    async fn dispatch_embeddings_for_ranges(
        &self,
        runtime: &EmbeddingDispatchRuntime,
        texts: &[String],
        model: Option<&str>,
        chunk_ranges: &[(usize, usize)],
        chunk_concurrency: usize,
    ) -> Option<Vec<Vec<f32>>> {
        if chunk_ranges.is_empty() {
            return Some(vec![]);
        }
        if chunk_ranges.len() == 1 {
            let vectors = dispatch_chunk_with_runtime(runtime, texts, model, 0, 1).await?;
            if vectors.len() != texts.len() {
                tracing::warn!(
                    event = "agent.embedding.batch.invalid_vector_count",
                    expected_vectors = texts.len(),
                    actual_vectors = vectors.len(),
                    chunk_count = 1,
                    "embedding backend returned unexpected vector count"
                );
                return None;
            }
            return Some(vectors);
        }

        if chunk_concurrency <= 1 {
            return self
                .dispatch_embeddings_sequential(runtime, texts, model, chunk_ranges)
                .await;
        }

        self.dispatch_embeddings_concurrent(runtime, texts, model, chunk_ranges, chunk_concurrency)
            .await
    }

    async fn dispatch_embeddings_sequential(
        &self,
        runtime: &EmbeddingDispatchRuntime,
        texts: &[String],
        model: Option<&str>,
        chunk_ranges: &[(usize, usize)],
    ) -> Option<Vec<Vec<f32>>> {
        let chunk_count = chunk_ranges.len();
        let mut merged = Vec::with_capacity(texts.len());
        for (chunk_index, (start, end)) in chunk_ranges.iter().copied().enumerate() {
            let chunk = &texts[start..end];
            let vectors =
                dispatch_chunk_with_runtime(runtime, chunk, model, chunk_index, chunk_count)
                    .await?;
            if vectors.len() != chunk.len() {
                tracing::warn!(
                    event = "agent.embedding.batch.invalid_chunk_vector_count",
                    chunk_index = chunk_index + 1,
                    chunk_count,
                    expected_vectors = chunk.len(),
                    actual_vectors = vectors.len(),
                    "embedding backend returned unexpected chunk vector count"
                );
                return None;
            }
            merged.extend(vectors);
        }
        Some(merged)
    }

    #[allow(clippy::too_many_lines)]
    async fn dispatch_embeddings_concurrent(
        &self,
        runtime: &EmbeddingDispatchRuntime,
        texts: &[String],
        model: Option<&str>,
        chunk_ranges: &[(usize, usize)],
        chunk_concurrency: usize,
    ) -> Option<Vec<Vec<f32>>> {
        let chunk_count = chunk_ranges.len();
        let concurrency = chunk_concurrency.max(1).min(chunk_count);
        tracing::debug!(
            event = "agent.embedding.batch.concurrent.start",
            chunk_count,
            chunk_concurrency = concurrency,
            "embedding chunked concurrent execution started"
        );

        let mut next_chunk = 0usize;
        let mut finished = 0usize;
        let mut pending = tokio::task::JoinSet::new();
        let mut chunk_results: Vec<Option<Vec<Vec<f32>>>> = vec![None; chunk_count];
        let model_owned = model.map(ToString::to_string);

        while next_chunk < concurrency {
            let (start, end) = chunk_ranges[next_chunk];
            let chunk_texts = texts[start..end].to_vec();
            pending.spawn(dispatch_chunk_with_runtime_owned(
                runtime.clone(),
                chunk_texts,
                model_owned.clone(),
                next_chunk,
                chunk_count,
            ));
            next_chunk += 1;
        }

        while finished < chunk_count {
            match pending.join_next().await {
                Some(Ok((chunk_index, vectors))) => {
                    finished = finished.saturating_add(1);
                    let (start, end) = chunk_ranges[chunk_index];
                    let expected_vectors = end - start;
                    let Some(vectors) = vectors else {
                        tracing::warn!(
                            event = "agent.embedding.batch.chunk_failed",
                            chunk_index = chunk_index + 1,
                            chunk_count,
                            "embedding chunk failed during concurrent execution"
                        );
                        pending.abort_all();
                        while pending.join_next().await.is_some() {}
                        return None;
                    };
                    if vectors.len() != expected_vectors {
                        tracing::warn!(
                            event = "agent.embedding.batch.invalid_chunk_vector_count",
                            chunk_index = chunk_index + 1,
                            chunk_count,
                            expected_vectors,
                            actual_vectors = vectors.len(),
                            "embedding backend returned unexpected chunk vector count"
                        );
                        pending.abort_all();
                        while pending.join_next().await.is_some() {}
                        return None;
                    }
                    chunk_results[chunk_index] = Some(vectors);
                    if next_chunk < chunk_count {
                        let (start, end) = chunk_ranges[next_chunk];
                        let chunk_texts = texts[start..end].to_vec();
                        pending.spawn(dispatch_chunk_with_runtime_owned(
                            runtime.clone(),
                            chunk_texts,
                            model_owned.clone(),
                            next_chunk,
                            chunk_count,
                        ));
                        next_chunk += 1;
                    }
                }
                Some(Err(error)) => {
                    tracing::warn!(
                        event = "agent.embedding.batch.chunk_join_failed",
                        chunk_count,
                        error = %error,
                        "embedding chunk task join failed"
                    );
                    pending.abort_all();
                    while pending.join_next().await.is_some() {}
                    return None;
                }
                None => {
                    tracing::warn!(
                        event = "agent.embedding.batch.chunk_join_unexpected_none",
                        chunk_count,
                        finished,
                        "embedding chunk join set ended unexpectedly"
                    );
                    return None;
                }
            }
        }

        let mut merged = Vec::with_capacity(texts.len());
        for (chunk_index, chunk_vectors) in chunk_results.into_iter().enumerate() {
            let Some(vectors) = chunk_vectors else {
                tracing::warn!(
                    event = "agent.embedding.batch.chunk_missing_result",
                    chunk_index = chunk_index + 1,
                    chunk_count,
                    "embedding chunk result missing after concurrent execution"
                );
                return None;
            };
            merged.extend(vectors);
        }
        tracing::debug!(
            event = "agent.embedding.batch.concurrent.completed",
            chunk_count,
            chunk_concurrency = concurrency,
            merged_vectors = merged.len(),
            "embedding chunked concurrent execution completed"
        );
        Some(merged)
    }

    /// Embed single text with an optional embedding model hint.
    pub async fn embed_with_model(&self, text: &str, model: Option<&str>) -> Option<Vec<f32>> {
        let texts = [text.to_string()];
        self.embed_batch_with_model(&texts, model)
            .await
            .and_then(|batch| batch.into_iter().next())
    }
}

async fn dispatch_chunk_with_runtime_owned(
    runtime: EmbeddingDispatchRuntime,
    texts: Vec<String>,
    model: Option<String>,
    chunk_index: usize,
    chunk_count: usize,
) -> (usize, Option<Vec<Vec<f32>>>) {
    let result =
        dispatch_chunk_with_runtime(&runtime, &texts, model.as_deref(), chunk_index, chunk_count)
            .await;
    (chunk_index, result)
}

#[allow(clippy::too_many_lines)]
async fn dispatch_chunk_with_runtime(
    runtime: &EmbeddingDispatchRuntime,
    texts: &[String],
    model: Option<&str>,
    chunk_index: usize,
    chunk_count: usize,
) -> Option<Vec<Vec<f32>>> {
    let gate_wait_started = Instant::now();
    let gate_available_before = runtime
        .in_flight_gate
        .as_ref()
        .map_or(0usize, |gate| gate.available_permits());
    let _in_flight_permit = if let Some(gate) = runtime.in_flight_gate.as_ref() {
        match gate.clone().acquire_owned().await {
            Ok(permit) => Some(permit),
            Err(error) => {
                tracing::warn!(
                    event = "agent.embedding.in_flight_gate.closed",
                    error = %error,
                    "embedding in-flight gate closed unexpectedly"
                );
                return None;
            }
        }
    } else {
        None
    };
    let gate_wait_ms = u64::try_from(gate_wait_started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let gate_available_after = runtime
        .in_flight_gate
        .as_ref()
        .map_or(0usize, |gate| gate.available_permits());
    tracing::debug!(
        event = "agent.embedding.batch.dispatch",
        backend = runtime.backend_mode.as_str(),
        backend_source = runtime.backend_source,
        chunk_index = chunk_index + 1,
        chunk_count,
        chunk_size = texts.len(),
        model = model.unwrap_or(""),
        max_in_flight = runtime.max_in_flight,
        gate_wait_ms,
        gate_available_before,
        gate_available_after,
        "dispatching embedding batch chunk request"
    );

    match runtime.backend_mode {
        EmbeddingBackendMode::Http => {
            let primary =
                embed_http(&runtime.client, runtime.base_url.as_str(), texts, model).await;
            if primary.is_some() {
                primary
            } else {
                embed_mcp(&runtime.client, runtime.mcp_url.as_deref(), texts).await
            }
        }
        EmbeddingBackendMode::OpenAiHttp => {
            let primary =
                embed_openai_http(&runtime.client, runtime.base_url.as_str(), texts, model).await;
            if primary.is_some() {
                primary
            } else {
                embed_mcp(&runtime.client, runtime.mcp_url.as_deref(), texts).await
            }
        }
        EmbeddingBackendMode::LiteLlmRs => {
            #[cfg(not(feature = "agent-provider-litellm"))]
            {
                tracing::warn!(
                    event = "agent.embedding.litellm.disabled",
                    "embedding backend resolved to litellm-rs but feature agent-provider-litellm is disabled; falling back to http/mcp"
                );
                let primary =
                    embed_http(&runtime.client, runtime.base_url.as_str(), texts, model).await;
                if primary.is_some() {
                    return primary;
                }
                return embed_mcp(&runtime.client, runtime.mcp_url.as_deref(), texts).await;
            }

            #[cfg(feature = "agent-provider-litellm")]
            {
                let Some(model) = model else {
                    tracing::warn!(
                        event = "agent.embedding.litellm.missing_model",
                        "embedding backend is litellm-rs but no model is configured"
                    );
                    return None;
                };
                if model.starts_with("ollama/") {
                    let http_fast_path = embed_http(
                        &runtime.client,
                        runtime.base_url.as_str(),
                        texts,
                        Some(model),
                    )
                    .await;
                    if http_fast_path.is_some() {
                        tracing::debug!(
                            event = "agent.embedding.ollama.http_fast_path.hit",
                            model,
                            base_url = runtime.base_url,
                            "ollama embedding served via /embed/batch fast path"
                        );
                        return http_fast_path;
                    }
                    tracing::debug!(
                        event = "agent.embedding.ollama.http_fast_path.miss",
                        model,
                        base_url = runtime.base_url,
                        "ollama /embed/batch fast path missed; trying litellm-rs"
                    );
                }
                let litellm = embed_litellm(
                    model,
                    texts,
                    runtime.base_url.as_str(),
                    runtime.timeout_secs,
                    runtime.litellm_api_key.as_deref(),
                )
                .await;
                if litellm.is_some() {
                    return litellm;
                }
                if model.starts_with("ollama/") {
                    tracing::warn!(
                        event = "agent.embedding.litellm.ollama_http_fallback",
                        model,
                        base_url = runtime.base_url,
                        "litellm-rs ollama embedding failed; retrying /embed/batch before MCP fallback"
                    );
                    let http_fallback = embed_http(
                        &runtime.client,
                        runtime.base_url.as_str(),
                        texts,
                        Some(model),
                    )
                    .await;
                    if http_fallback.is_some() {
                        return http_fallback;
                    }
                    return embed_mcp(&runtime.client, runtime.mcp_url.as_deref(), texts).await;
                }
                litellm
            }
        }
    }
}

fn build_chunk_ranges(total: usize, max_chunk_size: usize) -> Vec<(usize, usize)> {
    if total == 0 {
        return Vec::new();
    }
    let chunk = max_chunk_size.max(1);
    let mut ranges = Vec::with_capacity(total.div_ceil(chunk));
    let mut start = 0usize;
    while start < total {
        let end = (start + chunk).min(total);
        ranges.push((start, end));
        start = end;
    }
    ranges
}

fn build_http_client(timeout_secs: u64) -> reqwest::Client {
    let builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(64)
        .tcp_nodelay(true);
    match builder.build() {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to build tuned embedding http client; falling back to default client"
            );
            reqwest::Client::new()
        }
    }
}

#[cfg(feature = "agent-provider-litellm")]
fn resolve_litellm_embed_api_key() -> Option<String> {
    let from_env = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };

    from_env("OMNI_AGENT_EMBED_API_KEY")
        .or_else(|| from_env("LITELLM_API_KEY"))
        .or_else(|| {
            let runtime_settings = load_runtime_settings();
            runtime_settings
                .inference
                .api_key_env
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .and_then(from_env)
        })
        .or_else(|| from_env("MINIMAX_API_KEY"))
        .or_else(|| from_env("OPENAI_API_KEY"))
}

fn parse_positive_env_u64(name: &str, default_value: u64, max_value: u64) -> u64 {
    let value = std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value);
    value.min(max_value)
}

fn parse_positive_env_usize(name: &str, default_value: usize, max_value: usize) -> usize {
    let value = std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value);
    value.min(max_value)
}
