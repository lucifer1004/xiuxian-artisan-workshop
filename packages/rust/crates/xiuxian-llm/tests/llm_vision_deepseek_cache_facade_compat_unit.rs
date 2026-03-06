//! `DeepSeek` cache wrapper/facade compatibility contracts at crate-level test boundary.

use std::sync::{Mutex, OnceLock};

use xiuxian_llm::test_support::{
    DeepseekCacheKeyInput, DeepseekCacheTestFacade, build_deepseek_cache_key_for_tests,
    deepseek_cache_layer_labels_for_tests, deepseek_local_cache_clear_for_tests,
    deepseek_local_cache_get_for_tests, deepseek_local_cache_set_with_max_entries_for_tests,
    deepseek_store_markdown_in_cache_for_tests, deepseek_valkey_get_with_for_tests,
    deepseek_valkey_set_with_for_tests, normalize_deepseek_cache_text_owned_for_tests,
    normalize_deepseek_cache_text_view_for_tests, normalize_deepseek_valkey_timeout_ms_for_tests,
};

const CACHE_KEY: &str = "compat-key";
const CACHE_PREFIX: &str = "compat";

fn cache_compat_guard() -> std::sync::MutexGuard<'static, ()> {
    static CACHE_COMPAT_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = CACHE_COMPAT_MUTEX.get_or_init(|| Mutex::new(()));
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[test]
fn cache_key_wrapper_matches_facade_output() {
    let input = DeepseekCacheKeyInput {
        model_root: "/models/deepseek-ocr-2",
        prompt: "markdown please",
        base_size: 1024,
        image_size: 768,
        crop_mode: true,
        max_new_tokens: 256,
        original: b"abc123",
    };
    let wrapper = build_deepseek_cache_key_for_tests(&input);
    let facade = DeepseekCacheTestFacade::build_cache_key(&input);
    assert_eq!(wrapper, facade);
}

#[test]
fn text_normalization_wrappers_match_facade() {
    let view_text = " \n# compat\t";
    assert_eq!(
        normalize_deepseek_cache_text_view_for_tests(view_text),
        DeepseekCacheTestFacade::normalize_text_view(view_text)
    );

    let owned_text = String::from("  content  ");
    assert_eq!(
        normalize_deepseek_cache_text_owned_for_tests(owned_text.clone()),
        DeepseekCacheTestFacade::normalize_text_owned(owned_text)
    );
}

#[test]
fn telemetry_label_and_timeout_wrappers_match_facade() {
    assert_eq!(
        deepseek_cache_layer_labels_for_tests(),
        DeepseekCacheTestFacade::cache_layer_labels()
    );
    assert_eq!(
        normalize_deepseek_valkey_timeout_ms_for_tests(0),
        DeepseekCacheTestFacade::normalize_valkey_timeout_ms(0)
    );
    assert_eq!(
        normalize_deepseek_valkey_timeout_ms_for_tests(250),
        DeepseekCacheTestFacade::normalize_valkey_timeout_ms(250)
    );
}

#[test]
fn local_cache_wrappers_match_facade() {
    let _guard = cache_compat_guard();
    deepseek_local_cache_clear_for_tests();
    DeepseekCacheTestFacade::local_clear();

    deepseek_local_cache_set_with_max_entries_for_tests(CACHE_KEY, "wrapper-value", 4);
    assert_eq!(
        deepseek_local_cache_get_for_tests(CACHE_KEY),
        DeepseekCacheTestFacade::local_get(CACHE_KEY)
    );

    DeepseekCacheTestFacade::local_clear();
    DeepseekCacheTestFacade::local_set_with_max_entries(CACHE_KEY, "facade-value", 4);
    assert_eq!(
        deepseek_local_cache_get_for_tests(CACHE_KEY),
        DeepseekCacheTestFacade::local_get(CACHE_KEY)
    );

    deepseek_local_cache_clear_for_tests();
}

#[test]
fn write_and_valkey_wrappers_match_facade_paths() {
    let _guard = cache_compat_guard();
    deepseek_local_cache_clear_for_tests();

    deepseek_store_markdown_in_cache_for_tests(CACHE_KEY, "  ## title");
    let wrapper_cached = deepseek_local_cache_get_for_tests(CACHE_KEY);

    DeepseekCacheTestFacade::local_clear();
    DeepseekCacheTestFacade::store_markdown(CACHE_KEY, "  ## title");
    let facade_cached = DeepseekCacheTestFacade::local_get(CACHE_KEY);
    assert_eq!(wrapper_cached, facade_cached);

    let invalid_url = "://invalid";
    assert_eq!(
        deepseek_valkey_get_with_for_tests(invalid_url, CACHE_PREFIX, 60, 10, CACHE_KEY),
        DeepseekCacheTestFacade::valkey_get_with(invalid_url, CACHE_PREFIX, 60, 10, CACHE_KEY)
    );
    assert_eq!(
        deepseek_valkey_set_with_for_tests(
            invalid_url,
            CACHE_PREFIX,
            60,
            10,
            CACHE_KEY,
            "markdown"
        ),
        DeepseekCacheTestFacade::valkey_set_with(
            invalid_url,
            CACHE_PREFIX,
            60,
            10,
            CACHE_KEY,
            "markdown"
        )
    );

    deepseek_local_cache_clear_for_tests();
}
