//! Shared warmup option resolution used by CLI runtime and tests.

use crate::RuntimeSettings;

const DEFAULT_MEMORY_EMBED_BASE_URL: &str = "http://127.0.0.1:3002";
const DEFAULT_EMBED_TIMEOUT_SECS: u64 = 15;

/// Runtime + env override bag for embedding warmup resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WarmupEnvOverrides {
    pub memory_embedding_backend: Option<String>,
    pub embedding_backend: Option<String>,
    pub llm_backend: Option<String>,
    pub memory_embedding_model: Option<String>,
    pub embedding_model: Option<String>,
    pub memory_embedding_base_url: Option<String>,
    pub embedding_base_url: Option<String>,
    pub embed_timeout_secs: Option<u64>,
    pub memory_embed_batch_max_size: Option<usize>,
    pub embed_batch_max_size: Option<usize>,
    pub memory_embed_batch_max_concurrency: Option<usize>,
    pub embed_batch_max_concurrency: Option<usize>,
}

/// Resolved warmup execution options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarmupOptions {
    pub backend_hint: Option<String>,
    pub model: Option<String>,
    pub base_url: String,
    pub timeout_secs: u64,
    pub batch_max_size: Option<usize>,
    pub batch_max_concurrency: Option<usize>,
}

/// Resolve warmup options with deterministic precedence:
/// CLI model override -> env overrides -> runtime settings -> defaults.
#[must_use]
pub fn resolve_warmup_options(
    runtime_settings: &RuntimeSettings,
    env: &WarmupEnvOverrides,
    model_override: Option<&str>,
) -> WarmupOptions {
    let backend_hint = first_non_empty([
        env.memory_embedding_backend.clone(),
        trim_non_empty(runtime_settings.memory.embedding_backend.as_deref()),
        env.embedding_backend.clone(),
        trim_non_empty(runtime_settings.embedding.backend.as_deref()),
        env.llm_backend.clone(),
        trim_non_empty(runtime_settings.agent.llm_backend.as_deref()),
    ]);

    let model = trim_non_empty(model_override)
        .or_else(|| env.memory_embedding_model.clone())
        .or_else(|| trim_non_empty(runtime_settings.memory.embedding_model.as_deref()))
        .or_else(|| env.embedding_model.clone())
        .or_else(|| trim_non_empty(runtime_settings.embedding.litellm_model.as_deref()))
        .or_else(|| trim_non_empty(runtime_settings.embedding.model.as_deref()));

    let base_url = first_non_empty([
        env.memory_embedding_base_url.clone(),
        trim_non_empty(runtime_settings.memory.embedding_base_url.as_deref()),
        env.embedding_base_url.clone(),
        trim_non_empty(runtime_settings.embedding.client_url.as_deref()),
        trim_non_empty(runtime_settings.embedding.litellm_api_base.as_deref()),
    ])
    .unwrap_or_else(|| DEFAULT_MEMORY_EMBED_BASE_URL.to_string());

    let timeout_secs = env
        .embed_timeout_secs
        .or(runtime_settings.embedding.timeout_secs)
        .unwrap_or(DEFAULT_EMBED_TIMEOUT_SECS);

    let batch_max_size = env
        .memory_embed_batch_max_size
        .or(env.embed_batch_max_size)
        .or(runtime_settings
            .embedding
            .batch_max_size
            .filter(|value| *value > 0));

    let batch_max_concurrency = env
        .memory_embed_batch_max_concurrency
        .or(env.embed_batch_max_concurrency)
        .or(runtime_settings
            .embedding
            .batch_max_concurrency
            .filter(|value| *value > 0));

    WarmupOptions {
        backend_hint,
        model,
        base_url,
        timeout_secs,
        batch_max_size,
        batch_max_concurrency,
    }
}

fn first_non_empty<const N: usize>(values: [Option<String>; N]) -> Option<String> {
    values.into_iter().flatten().next()
}

fn trim_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(ToString::to_string)
}
