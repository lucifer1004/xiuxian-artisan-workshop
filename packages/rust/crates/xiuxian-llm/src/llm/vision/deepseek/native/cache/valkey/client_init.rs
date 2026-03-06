use std::sync::Arc;

use tracing::warn;

use super::super::super::super::util::sanitize_error_string;
use super::super::super::env::{cache_key_prefix, cache_valkey_url, parse_env_u64};
use super::ops::ValkeyOcrCache;

pub(super) fn load_valkey_cache() -> Option<ValkeyOcrCache> {
    // Optional distributed cache (Valkey):
    // - XIUXIAN_VISION_OCR_CACHE_VALKEY_URL: enable switch when non-empty
    // - XIUXIAN_VISION_OCR_CACHE_PREFIX: key namespace (default xiuxian:vision:ocr:v1)
    // - XIUXIAN_VISION_OCR_CACHE_TTL_SECS: key ttl (default 3600)
    // - XIUXIAN_VISION_OCR_CACHE_TIMEOUT_MS: socket read/write timeout (default 200)
    let valkey_url = cache_valkey_url()?;
    let key_prefix = cache_key_prefix().unwrap_or_else(|| "xiuxian:vision:ocr:v1".to_string());
    let ttl_secs = parse_env_u64("XIUXIAN_VISION_OCR_CACHE_TTL_SECS").unwrap_or(3_600);
    let io_timeout_ms = normalize_io_timeout_ms(
        parse_env_u64("XIUXIAN_VISION_OCR_CACHE_TIMEOUT_MS").unwrap_or(200),
    );
    build_valkey_cache(
        valkey_url.as_str(),
        key_prefix.as_str(),
        ttl_secs,
        io_timeout_ms,
    )
}

pub(super) fn build_valkey_cache(
    valkey_url: &str,
    key_prefix: &str,
    ttl_secs: u64,
    io_timeout_ms: u64,
) -> Option<ValkeyOcrCache> {
    let client = match redis::Client::open(valkey_url) {
        Ok(client) => client,
        Err(error) => {
            warn!(
                event = "llm.vision.deepseek.valkey.init_failed",
                error = %sanitize_error_string(error),
                "DeepSeek OCR Valkey cache disabled because initialization failed"
            );
            return None;
        }
    };
    Some(ValkeyOcrCache {
        client,
        key_prefix: Arc::from(key_prefix.to_string()),
        ttl_secs,
        io_timeout_ms: normalize_io_timeout_ms(io_timeout_ms),
    })
}

pub(super) fn normalize_io_timeout_ms(io_timeout_ms: u64) -> u64 {
    io_timeout_ms.max(1)
}
