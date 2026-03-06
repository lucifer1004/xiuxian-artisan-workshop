//! `DeepSeek` cache-layer label mapping invariants at crate-level test boundary.

use xiuxian_llm::test_support::deepseek_cache_layer_labels_for_tests;

#[test]
fn cache_layer_labels_match_expected_telemetry_tokens() {
    let labels = deepseek_cache_layer_labels_for_tests();
    assert_eq!(labels, ("local", "valkey"));
}
