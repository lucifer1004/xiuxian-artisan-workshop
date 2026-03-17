mod client_init;
mod ops;

pub fn set(key: &str, markdown: &str) -> bool {
    if let Some(cache) = client_init::load_valkey_cache() {
        cache.set(key, markdown)
    } else {
        false
    }
}

pub fn get_with(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    let cache = client_init::acquire_client(valkey_url, key_prefix, ttl_secs, io_timeout_ms);
    cache.and_then(|cache| cache.get(key))
}

pub fn set_with(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    let cache = client_init::acquire_client(valkey_url, key_prefix, ttl_secs, io_timeout_ms);
    cache.is_some_and(|cache| cache.set(key, markdown))
}

pub fn normalize_valkey_timeout_ms_for_tests(io_timeout_ms: u64) -> u64 {
    client_init::normalize_io_timeout_ms(io_timeout_ms)
}
