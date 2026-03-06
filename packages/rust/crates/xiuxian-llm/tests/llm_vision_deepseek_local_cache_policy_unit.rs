//! `DeepSeek` local-cache policy invariants at crate-level test boundary.

use std::sync::{Mutex, OnceLock};

use xiuxian_llm::test_support::{
    deepseek_local_cache_clear_for_tests, deepseek_local_cache_get_for_tests,
    deepseek_local_cache_set_with_max_entries_for_tests,
};

fn local_cache_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCAL_CACHE_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = LOCAL_CACHE_TEST_MUTEX.get_or_init(|| Mutex::new(()));
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[test]
fn local_cache_clears_before_insert_when_capacity_is_reached() {
    let _guard = local_cache_test_guard();
    deepseek_local_cache_clear_for_tests();

    deepseek_local_cache_set_with_max_entries_for_tests("k1", "v1", 2);
    deepseek_local_cache_set_with_max_entries_for_tests("k2", "v2", 2);

    assert_eq!(
        deepseek_local_cache_get_for_tests("k1").as_deref(),
        Some("v1")
    );
    assert_eq!(
        deepseek_local_cache_get_for_tests("k2").as_deref(),
        Some("v2")
    );

    deepseek_local_cache_set_with_max_entries_for_tests("k3", "v3", 2);

    assert_eq!(deepseek_local_cache_get_for_tests("k1"), None);
    assert_eq!(deepseek_local_cache_get_for_tests("k2"), None);
    assert_eq!(
        deepseek_local_cache_get_for_tests("k3").as_deref(),
        Some("v3")
    );
}

#[test]
fn local_cache_normalizes_zero_capacity_to_one_entry() {
    let _guard = local_cache_test_guard();
    deepseek_local_cache_clear_for_tests();

    deepseek_local_cache_set_with_max_entries_for_tests("first", "v1", 0);
    assert_eq!(
        deepseek_local_cache_get_for_tests("first").as_deref(),
        Some("v1")
    );

    deepseek_local_cache_set_with_max_entries_for_tests("second", "v2", 0);

    assert_eq!(deepseek_local_cache_get_for_tests("first"), None);
    assert_eq!(
        deepseek_local_cache_get_for_tests("second").as_deref(),
        Some("v2")
    );
}
