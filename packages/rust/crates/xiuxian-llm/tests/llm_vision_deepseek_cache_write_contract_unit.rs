//! `DeepSeek` cache-write contract invariants at crate-level test boundary.

use std::sync::{Mutex, OnceLock};

use xiuxian_llm::test_support::{
    deepseek_local_cache_clear_for_tests, deepseek_local_cache_get_for_tests,
    deepseek_store_markdown_in_cache_for_tests,
};

fn cache_write_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static CACHE_WRITE_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = CACHE_WRITE_TEST_MUTEX.get_or_init(|| Mutex::new(()));
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[test]
fn cache_write_skips_empty_markdown_payloads() {
    let _guard = cache_write_test_guard();
    deepseek_local_cache_clear_for_tests();

    deepseek_store_markdown_in_cache_for_tests("cache-empty", "");

    assert_eq!(deepseek_local_cache_get_for_tests("cache-empty"), None);
}

#[test]
fn cache_write_stores_non_empty_markdown_payloads() {
    let _guard = cache_write_test_guard();
    deepseek_local_cache_clear_for_tests();
    let markdown = "  # title\n\ncontent";

    deepseek_store_markdown_in_cache_for_tests("cache-present", markdown);

    assert_eq!(
        deepseek_local_cache_get_for_tests("cache-present").as_deref(),
        Some(markdown)
    );
}
