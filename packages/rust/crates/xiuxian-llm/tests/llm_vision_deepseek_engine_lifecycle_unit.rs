//! `DeepSeek` OCR engine lifecycle selection tests.

use xiuxian_llm::test_support::{
    resolve_deepseek_engine_device_label_for_tests, should_reuse_deepseek_engine_cache_for_tests,
};

#[test]
fn cpu_fallback_forces_cpu_engine_selection() {
    assert_eq!(
        resolve_deepseek_engine_device_label_for_tests("metal", true),
        "cpu"
    );
    assert_eq!(
        resolve_deepseek_engine_device_label_for_tests("cuda", true),
        "cpu"
    );
}

#[test]
fn cache_reuse_rejects_prior_metal_engine_after_cpu_fallback() {
    assert!(!should_reuse_deepseek_engine_cache_for_tests(
        "/tmp/model",
        "metal",
        "/tmp/model",
        "metal",
        true,
    ));
    assert!(should_reuse_deepseek_engine_cache_for_tests(
        "/tmp/model",
        "cpu",
        "/tmp/model",
        "metal",
        true,
    ));
}

#[test]
fn cache_reuse_requires_matching_model_root() {
    assert!(!should_reuse_deepseek_engine_cache_for_tests(
        "/tmp/model-a",
        "cpu",
        "/tmp/model-b",
        "cpu",
        false,
    ));
}
