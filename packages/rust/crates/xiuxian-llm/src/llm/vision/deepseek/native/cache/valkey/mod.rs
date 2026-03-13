mod client_init;
mod ops;

pub use self::client_init::{
    acquire_client, build_valkey_cache, load_valkey_cache, normalize_io_timeout_ms,
};
pub use self::ops::ValkeyOcrCache;

pub fn get(key: &str) -> Option<String> {
    if let Some(cache) = client_init::load_valkey_cache() {
        cache.get(key)
    } else {
        None
    }
}

pub fn set(key: &str, markdown: &str) -> bool {
    if let Some(cache) = client_init::load_valkey_cache() {
        cache.set(key, markdown)
    } else {
        false
    }
}

pub fn valkey_get_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    let cache = client_init::acquire_client(valkey_url, key_prefix, ttl_secs, io_timeout_ms);
    cache.get(key)
}

pub fn valkey_set_with_for_tests(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    let cache = client_init::acquire_client(valkey_url, key_prefix, ttl_secs, io_timeout_ms);
    cache.set(key, markdown)
}

pub fn normalize_valkey_timeout_ms_for_tests(io_timeout_ms: u64) -> u64 {
    client_init::normalize_io_timeout_ms(io_timeout_ms)
}
