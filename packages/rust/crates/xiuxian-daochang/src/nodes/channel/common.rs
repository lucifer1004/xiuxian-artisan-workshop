use crate::config::RuntimeSettings;
use xiuxian_llm::embedding::backend::{EmbeddingBackendKind, parse_embedding_backend_kind};
use xiuxian_macros::env_non_empty;

fn trim_non_empty(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn apply_channel_embedding_memory_guard(
    runtime_settings: &RuntimeSettings,
    provider: &str,
) -> RuntimeSettings {
    apply_channel_embedding_memory_guard_for_tests(
        runtime_settings,
        env_non_empty!("OMNI_AGENT_MEMORY_EMBEDDING_BACKEND").as_deref(),
        env_non_empty!("OMNI_AGENT_EMBED_BACKEND").as_deref(),
        false,
        provider,
    )
}

#[cfg(test)]
pub(super) fn apply_channel_embedding_memory_guard_for_tests(
    runtime_settings: &RuntimeSettings,
    env_memory_backend: Option<&str>,
    env_embed_backend: Option<&str>,
    allow_inproc_embed: bool,
    provider: &str,
) -> RuntimeSettings {
    let mut guarded = runtime_settings.clone();

    if let Some(backend) = trim_non_empty(env_memory_backend) {
        guarded.memory.embedding_backend = Some(backend);
        return guarded;
    }

    if let Some(backend) = trim_non_empty(env_embed_backend) {
        guarded.memory.embedding_backend = Some(backend);
        return guarded;
    }

    if allow_inproc_embed {
        return guarded;
    }

    let configured_backend = guarded
        .memory
        .embedding_backend
        .as_deref()
        .or(guarded.embedding.backend.as_deref());
    if !matches!(
        parse_embedding_backend_kind(configured_backend),
        Some(EmbeddingBackendKind::MistralSdk)
    ) {
        return guarded;
    }

    guarded.memory.embedding_backend = Some(EmbeddingBackendKind::Http.as_str().to_string());

    tracing::warn!(
        event = "channel.embedding.memory_guard",
        provider = %provider,
        from_backend = "mistral_sdk",
        to_backend = "http",
        "forcing channel embedding backend to http to avoid in-process mistral_sdk memory spikes"
    );

    guarded
}

pub(super) fn parse_comma_separated_entries(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn parse_optional_comma_separated_entries(raw: Option<String>) -> Option<Vec<String>> {
    raw.map(|value| parse_comma_separated_entries(&value))
}

pub(super) fn parse_semicolon_separated_entries(raw: &str) -> Vec<String> {
    raw.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn log_control_command_allow_override(provider: &str, entries: &Option<Vec<String>>) {
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

pub(super) fn log_slash_command_allow_override(provider: &str, entries: &Option<Vec<String>>) {
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
