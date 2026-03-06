use std::time::Instant;

use anyhow::{Result, anyhow};
use xiuxian_daochang::warmup_options::{WarmupEnvOverrides, resolve_warmup_options};
use xiuxian_daochang::{EmbeddingClient, RuntimeSettings};
use xiuxian_llm::embedding::backend::{EmbeddingBackendKind, parse_embedding_backend_kind};

use crate::resolve::{parse_positive_u64_from_env, parse_positive_usize_from_env};

const MISTRAL_SDK_INPROC_LABEL: &str = "inproc://mistral-sdk";

pub(crate) async fn run_embedding_warmup(
    runtime_settings: &RuntimeSettings,
    text: String,
    model_override: Option<String>,
    mistral_sdk_only: bool,
) -> Result<()> {
    let env = warmup_env_overrides_from_process_env();
    let options = resolve_warmup_options(runtime_settings, &env, model_override.as_deref());
    let backend_kind = parse_embedding_backend_kind(options.backend_hint.as_deref());
    let display_base_url = if matches!(backend_kind, Some(EmbeddingBackendKind::MistralSdk)) {
        MISTRAL_SDK_INPROC_LABEL
    } else {
        options.base_url.as_str()
    };

    if mistral_sdk_only && !matches!(backend_kind, Some(EmbeddingBackendKind::MistralSdk)) {
        println!(
            "Embedding warmup skipped: effective backend='{}' is not mistral_sdk",
            options.backend_hint.as_deref().unwrap_or("auto")
        );
        return Ok(());
    }

    println!(
        "Embedding warmup starting: backend='{}' model='{}' timeout_secs={} base_url='{}'",
        options.backend_hint.as_deref().unwrap_or("auto"),
        options.model.as_deref().unwrap_or("<default>"),
        options.timeout_secs,
        display_base_url
    );
    if matches!(backend_kind, Some(EmbeddingBackendKind::MistralSdk)) {
        println!(
            "Mistral SDK cache: hf_cache_path='{}' hf_revision='{}'",
            options
                .mistral_sdk_hf_cache_path
                .as_deref()
                .unwrap_or("<default>"),
            options
                .mistral_sdk_hf_revision
                .as_deref()
                .unwrap_or("<default>")
        );
        println!("Mistral SDK transport: in-process Rust runtime (HTTP base_url ignored).");
    }

    let client = EmbeddingClient::new_with_backend_and_tuning(
        options.base_url.as_str(),
        options.timeout_secs,
        options.backend_hint.as_deref(),
        options.batch_max_size,
        options.batch_max_concurrency,
    );
    let started = Instant::now();
    let maybe_vector = client
        .embed_with_model(text.as_str(), options.model.as_deref())
        .await;
    let elapsed_ms = started.elapsed().as_millis();
    match maybe_vector {
        Some(vector) => {
            println!(
                "Embedding warmup succeeded: dim={} elapsed_ms={elapsed_ms}",
                vector.len()
            );
            Ok(())
        }
        None => Err(anyhow!(
            "embedding warmup failed: backend='{}' model='{}' base_url='{}'",
            options.backend_hint.as_deref().unwrap_or("auto"),
            options.model.as_deref().unwrap_or("<default>"),
            options.base_url
        )),
    }
}

fn warmup_env_overrides_from_process_env() -> WarmupEnvOverrides {
    WarmupEnvOverrides {
        memory_embedding_backend: non_empty_env("OMNI_AGENT_MEMORY_EMBEDDING_BACKEND"),
        embedding_backend: non_empty_env("OMNI_AGENT_EMBED_BACKEND"),
        llm_backend: non_empty_env("OMNI_AGENT_LLM_BACKEND"),
        memory_embedding_model: non_empty_env("OMNI_AGENT_MEMORY_EMBEDDING_MODEL"),
        embedding_model: non_empty_env("OMNI_AGENT_EMBED_MODEL"),
        memory_embedding_base_url: non_empty_env("OMNI_AGENT_MEMORY_EMBEDDING_BASE_URL"),
        embedding_base_url: non_empty_env("OMNI_AGENT_EMBED_BASE_URL"),
        embed_timeout_secs: parse_positive_u64_from_env("OMNI_AGENT_EMBED_TIMEOUT_SECS"),
        memory_embed_batch_max_size: parse_positive_usize_from_env(
            "OMNI_AGENT_MEMORY_EMBED_BATCH_MAX_SIZE",
        ),
        embed_batch_max_size: parse_positive_usize_from_env("OMNI_AGENT_EMBED_BATCH_MAX_SIZE"),
        memory_embed_batch_max_concurrency: parse_positive_usize_from_env(
            "OMNI_AGENT_MEMORY_EMBED_BATCH_MAX_CONCURRENCY",
        ),
        embed_batch_max_concurrency: parse_positive_usize_from_env(
            "OMNI_AGENT_EMBED_BATCH_MAX_CONCURRENCY",
        ),
        mistral_sdk_hf_cache_path: non_empty_env("OMNI_AGENT_MISTRAL_SDK_HF_CACHE_PATH"),
        mistral_sdk_hf_revision: non_empty_env("OMNI_AGENT_MISTRAL_SDK_HF_REVISION"),
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
