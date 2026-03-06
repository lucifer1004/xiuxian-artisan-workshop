use super::handlers::fallback_hash_embed_batch;
use super::runtime::apply_gateway_embedding_memory_guard_for_tests;
use crate::config::RuntimeSettings;

#[test]
fn hash_fallback_batch_preserves_count_and_dimension() {
    let inputs = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
    let vectors = fallback_hash_embed_batch(&inputs, 64);
    assert_eq!(vectors.len(), inputs.len());
    assert!(vectors.iter().all(|vector| vector.len() == 64));
}

#[test]
fn hash_fallback_is_deterministic_for_identical_input() {
    let first = fallback_hash_embed_batch(&["same-input".to_string()], 32);
    let second = fallback_hash_embed_batch(&["same-input".to_string()], 32);
    assert_eq!(first, second);
}

#[test]
fn gateway_forces_http_embedding_backend_when_runtime_defaults_to_mistral_sdk() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());

    let resolved =
        apply_gateway_embedding_memory_guard_for_tests(&settings, None, None, Some("false"));

    assert_eq!(resolved.memory.embedding_backend.as_deref(), Some("http"));
}

#[test]
fn gateway_respects_explicit_memory_backend_env_override() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());

    let resolved = apply_gateway_embedding_memory_guard_for_tests(
        &settings,
        Some("http"),
        None,
        Some("false"),
    );

    assert_eq!(resolved.memory.embedding_backend.as_deref(), Some("http"));
}

#[test]
fn gateway_respects_allow_inproc_embed_flag() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());

    let resolved =
        apply_gateway_embedding_memory_guard_for_tests(&settings, None, None, Some("true"));

    assert_eq!(
        resolved.memory.embedding_backend.as_deref(),
        Some("mistral_sdk")
    );
}
