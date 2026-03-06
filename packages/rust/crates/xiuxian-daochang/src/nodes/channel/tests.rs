use xiuxian_daochang::RuntimeSettings;

use super::common::apply_channel_embedding_memory_guard_for_tests;

#[test]
fn channel_forces_http_embedding_backend_when_runtime_defaults_to_mistral_sdk() {
    let mut settings = RuntimeSettings::default();
    settings.embedding.backend = Some("mistral_sdk".to_string());

    let resolved =
        apply_channel_embedding_memory_guard_for_tests(&settings, None, None, false, "telegram");

    assert_eq!(resolved.memory.embedding_backend.as_deref(), Some("http"));
}

#[test]
fn channel_respects_explicit_memory_backend_env_override() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());

    let resolved = apply_channel_embedding_memory_guard_for_tests(
        &settings,
        Some("mistral_sdk"),
        None,
        false,
        "telegram",
    );

    assert_eq!(
        resolved.memory.embedding_backend.as_deref(),
        Some("mistral_sdk")
    );
}

#[test]
fn channel_respects_allow_inproc_embed_flag() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());

    let resolved =
        apply_channel_embedding_memory_guard_for_tests(&settings, None, None, true, "discord");

    assert_eq!(
        resolved.memory.embedding_backend.as_deref(),
        Some("mistral_sdk")
    );
}
