#[must_use]
pub(super) fn get_with(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
) -> Option<String> {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::valkey_get_with_for_tests(
            valkey_url,
            key_prefix,
            ttl_secs,
            io_timeout_ms,
            key,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = valkey_url;
        let _ = key_prefix;
        let _ = ttl_secs;
        let _ = io_timeout_ms;
        let _ = key;
        None
    }
}

#[must_use]
pub(super) fn set_with(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
    key: &str,
    markdown: &str,
) -> bool {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::valkey_set_with_for_tests(
            valkey_url,
            key_prefix,
            ttl_secs,
            io_timeout_ms,
            key,
            markdown,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = valkey_url;
        let _ = key_prefix;
        let _ = ttl_secs;
        let _ = io_timeout_ms;
        let _ = key;
        let _ = markdown;
        false
    }
}

#[must_use]
pub(super) fn normalize_timeout_ms(io_timeout_ms: u64) -> u64 {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::normalize_valkey_timeout_ms_for_tests(io_timeout_ms)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        io_timeout_ms.max(1)
    }
}
