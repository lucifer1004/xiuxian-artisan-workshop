//! Verifies the default `xiuxian-llm` feature profile stays on the LiteLLM-only path.

#[test]
fn default_feature_profile_keeps_provider_litellm_enabled() {
    assert!(cfg!(feature = "provider-litellm"));
    let _ = std::mem::size_of::<xiuxian_llm::llm::OpenAICompatibleClient>();
    let backend = xiuxian_llm::embedding::backend::parse_embedding_backend_kind(Some("litellm"));
    assert_eq!(
        backend,
        Some(xiuxian_llm::embedding::backend::EmbeddingBackendKind::LiteLlmRs)
    );
}
