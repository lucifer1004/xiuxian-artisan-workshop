//! Verifies the default `xiuxian-llm` feature profile stays on the LiteLLM-only path.

#[cfg(feature = "local-llm")]
compile_error!("default xiuxian-llm build must not enable `local-llm`");

#[test]
fn default_feature_profile_keeps_local_llm_disabled() {
    assert!(!cfg!(feature = "local-llm"));
    assert!(!cfg!(feature = "mistral.rs"));
    assert!(!cfg!(feature = "local-llm-vision-dots"));
    assert!(!cfg!(feature = "vision-dots"));
    let _ = std::mem::size_of::<xiuxian_llm::llm::OpenAICompatibleClient>();
    let backend = xiuxian_llm::embedding::backend::parse_embedding_backend_kind(Some("litellm"));
    assert_eq!(
        backend,
        Some(xiuxian_llm::embedding::backend::EmbeddingBackendKind::LiteLlmRs)
    );
}
