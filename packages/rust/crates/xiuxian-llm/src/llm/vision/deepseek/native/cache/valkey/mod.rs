mod client_init;
mod ops;

use std::sync::OnceLock;

use self::ops::ValkeyOcrCache;

static VALKEY_CACHE: OnceLock<Option<ValkeyOcrCache>> = OnceLock::new();

pub(in crate::llm::vision::deepseek::native) fn valkey_get(key: &str) -> Option<String> {
    valkey_cache().and_then(|cache| cache.get(key))
}

pub(in crate::llm::vision::deepseek::native) fn valkey_set(key: &str, markdown: &str) {
    if let Some(cache) = valkey_cache() {
        let _ = cache.set(key, markdown);
    }
}

fn valkey_cache() -> Option<&'static ValkeyOcrCache> {
    VALKEY_CACHE
        .get_or_init(client_init::load_valkey_cache)
        .as_ref()
}

pub(in crate::llm::vision::deepseek::native) fn valkey_get_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    let cache = client_init::build_valkey_cache(valkey_url, key_prefix, ttl_secs, io_timeout_ms)?;
    cache.get(key)
}

pub(in crate::llm::vision::deepseek::native) fn valkey_set_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    let Some(cache) =
        client_init::build_valkey_cache(valkey_url, key_prefix, ttl_secs, io_timeout_ms)
    else {
        return false;
    };
    cache.set(key, markdown)
}

pub(in crate::llm::vision::deepseek::native) fn normalize_valkey_timeout_ms_for_tests(
    io_timeout_ms: u64,
) -> u64 {
    client_init::normalize_io_timeout_ms(io_timeout_ms)
}
