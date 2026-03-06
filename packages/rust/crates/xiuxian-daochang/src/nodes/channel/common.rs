use xiuxian_daochang::RuntimeSettings;
use xiuxian_daochang::run_deepseek_vision_startup_probe_once;
use xiuxian_llm::embedding::backend::{EmbeddingBackendKind, parse_embedding_backend_kind};
use xiuxian_macros::env_non_empty;

const CHANNEL_ALLOW_INPROC_EMBED_ENV: &str = "OMNI_AGENT_CHANNEL_ALLOW_INPROC_EMBED";

pub(super) fn log_control_command_allow_override(provider: &str, entries: Option<&[String]>) {
    if let Some(entries) = entries {
        if entries.is_empty() {
            tracing::warn!(
                provider = %provider,
                "{provider}.control_command_allow_from is configured but empty; privileged control commands are denied for all senders"
            );
        } else {
            tracing::info!(
                provider = %provider,
                entries = entries.len(),
                "{provider}.control_command_allow_from override is active"
            );
        }
    }
}

pub(super) fn apply_channel_embedding_memory_guard(
    runtime_settings: &RuntimeSettings,
    provider: &str,
) -> RuntimeSettings {
    apply_channel_embedding_memory_guard_with_inputs(
        runtime_settings,
        env_non_empty!("OMNI_AGENT_MEMORY_EMBEDDING_BACKEND").as_deref(),
        env_non_empty!("OMNI_AGENT_EMBED_BACKEND").as_deref(),
        channel_allow_inproc_embed(),
        provider,
    )
}

fn apply_channel_embedding_memory_guard_with_inputs(
    runtime_settings: &RuntimeSettings,
    env_memory_backend: Option<&str>,
    env_embed_backend: Option<&str>,
    allow_inproc_embed: bool,
    provider: &str,
) -> RuntimeSettings {
    if allow_inproc_embed {
        return runtime_settings.clone();
    }

    if env_memory_backend.is_some() || env_embed_backend.is_some() {
        return runtime_settings.clone();
    }

    let configured_backend = runtime_settings
        .memory
        .embedding_backend
        .as_deref()
        .or(runtime_settings.embedding.backend.as_deref());
    if !matches!(
        parse_embedding_backend_kind(configured_backend),
        Some(EmbeddingBackendKind::MistralSdk)
    ) {
        return runtime_settings.clone();
    }

    let mut guarded = runtime_settings.clone();
    guarded.memory.embedding_backend = Some("http".to_string());

    tracing::warn!(
        provider = %provider,
        event = "agent.channel.embedding.memory_guard",
        from_backend = "mistral_sdk",
        to_backend = "http",
        allow_inproc_embed_env = CHANNEL_ALLOW_INPROC_EMBED_ENV,
        "forcing channel memory embedding backend to http to avoid in-process mistral_sdk memory spikes"
    );

    guarded
}

fn channel_allow_inproc_embed() -> bool {
    env_non_empty!(CHANNEL_ALLOW_INPROC_EMBED_ENV)
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_some_and(|raw| matches!(raw.as_str(), "1" | "true" | "yes" | "on"))
}

#[cfg(test)]
pub(super) fn apply_channel_embedding_memory_guard_for_tests(
    runtime_settings: &RuntimeSettings,
    env_memory_backend: Option<&str>,
    env_embed_backend: Option<&str>,
    allow_inproc_embed: bool,
    provider: &str,
) -> RuntimeSettings {
    apply_channel_embedding_memory_guard_with_inputs(
        runtime_settings,
        env_memory_backend,
        env_embed_backend,
        allow_inproc_embed,
        provider,
    )
}

pub(super) fn log_slash_command_allow_override(provider: &str, entries: Option<&[String]>) {
    if let Some(entries) = entries {
        if entries.is_empty() {
            tracing::warn!(
                provider = %provider,
                "{provider}.slash_command_allow_from is configured but empty; managed slash commands are denied for all non-admin senders"
            );
        } else {
            tracing::info!(
                provider = %provider,
                entries = entries.len(),
                "{provider}.slash_command_allow_from override is active"
            );
        }
    }
}

pub(super) fn run_channel_vision_startup_warmup(provider: &'static str) {
    run_deepseek_vision_startup_probe_once(provider);
}
