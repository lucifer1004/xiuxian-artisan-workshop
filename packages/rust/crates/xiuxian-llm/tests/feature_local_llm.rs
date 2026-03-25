#![cfg(feature = "local-llm")]

//! Verifies the `local-llm` umbrella enables both `mistral.rs` and `vision-dots`.

#[test]
fn local_llm_feature_exposes_mistral_and_deepseek_surfaces() {
    assert!(cfg!(feature = "mistral.rs"));
    assert!(cfg!(feature = "local-llm-vision-dots"));
    assert!(cfg!(feature = "vision-dots"));
    let _ = std::mem::size_of::<xiuxian_llm::mistral::MistralServerConfig>();
    let _ = std::mem::size_of::<xiuxian_llm::llm::vision::DeepseekRuntime>();
}

#[test]
fn local_llm_feature_exposes_mistral_sdk_embedding_surface() {
    assert_eq!(
        xiuxian_llm::embedding::sdk::normalize_mistral_sdk_model(Some("  local/model  ")),
        Some("local/model".to_string())
    );
}
