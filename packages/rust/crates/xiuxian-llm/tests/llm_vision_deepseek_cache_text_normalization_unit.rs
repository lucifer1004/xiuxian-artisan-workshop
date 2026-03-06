//! `DeepSeek` cache-text normalization invariants at crate-level test boundary.

use xiuxian_llm::test_support::{
    normalize_deepseek_cache_text_owned_for_tests, normalize_deepseek_cache_text_view_for_tests,
};

#[test]
fn cache_text_view_normalization_trims_and_filters_empty_values() {
    assert_eq!(
        normalize_deepseek_cache_text_view_for_tests("  # title  ").as_deref(),
        Some("# title")
    );
    assert_eq!(normalize_deepseek_cache_text_view_for_tests(" \n\t "), None);
}

#[test]
fn cache_text_owned_normalization_preserves_pointer_when_already_trimmed() {
    let original = String::from("already-trimmed");
    let original_ptr = original.as_ptr();

    let normalized = normalize_deepseek_cache_text_owned_for_tests(original)
        .unwrap_or_else(|| panic!("expected non-empty normalized cache text"));

    assert_eq!(normalized, "already-trimmed");
    assert_eq!(normalized.as_ptr(), original_ptr);
}

#[test]
fn cache_text_owned_normalization_trims_and_filters_empty_values() {
    let normalized = normalize_deepseek_cache_text_owned_for_tests(String::from("\n  value \t"));
    assert_eq!(normalized.as_deref(), Some("value"));

    let empty = normalize_deepseek_cache_text_owned_for_tests(String::from(" \n "));
    assert_eq!(empty, None);
}
