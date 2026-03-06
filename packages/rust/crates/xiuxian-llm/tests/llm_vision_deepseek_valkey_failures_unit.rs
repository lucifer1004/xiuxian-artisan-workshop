//! `DeepSeek` Valkey failure-path contracts at crate-level test boundary.

use xiuxian_llm::test_support::{
    deepseek_valkey_get_with_for_tests, deepseek_valkey_set_with_for_tests,
    normalize_deepseek_valkey_timeout_ms_for_tests,
};

const KEY_PREFIX: &str = "xiuxian:vision:ocr:test";
const CACHE_KEY: &str = "sample";
const CACHE_VALUE: &str = "# markdown";

#[test]
fn valkey_helpers_handle_invalid_url_without_panicking() {
    let get_result =
        deepseek_valkey_get_with_for_tests("://invalid", KEY_PREFIX, 60, 10, CACHE_KEY);
    let set_result = deepseek_valkey_set_with_for_tests(
        "://invalid",
        KEY_PREFIX,
        60,
        10,
        CACHE_KEY,
        CACHE_VALUE,
    );

    assert!(get_result.is_none());
    assert!(!set_result);
}

#[test]
fn valkey_helpers_handle_connection_failure_without_panicking() {
    let unreachable_url = "redis://127.0.0.1:1/0";
    let get_result =
        deepseek_valkey_get_with_for_tests(unreachable_url, KEY_PREFIX, 60, 1, CACHE_KEY);
    let set_result = deepseek_valkey_set_with_for_tests(
        unreachable_url,
        KEY_PREFIX,
        60,
        1,
        CACHE_KEY,
        CACHE_VALUE,
    );

    assert!(get_result.is_none());
    assert!(!set_result);
}

#[test]
fn valkey_timeout_normalization_clamps_to_minimum_one_millisecond() {
    assert_eq!(normalize_deepseek_valkey_timeout_ms_for_tests(0), 1);
    assert_eq!(normalize_deepseek_valkey_timeout_ms_for_tests(250), 250);
}
