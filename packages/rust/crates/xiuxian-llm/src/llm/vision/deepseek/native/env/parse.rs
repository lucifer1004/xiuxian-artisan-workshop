use super::super::super::config;
use super::super::super::util::non_empty_env;

pub(in crate::llm::vision::deepseek::native) fn parse_env_u32(key: &str) -> Option<u32> {
    non_empty_env(key)
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_BASE_SIZE" => config::base_size(),
            "XIUXIAN_VISION_IMAGE_SIZE" => config::image_size(),
            _ => None,
        })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_usize(key: &str) -> Option<usize> {
    non_empty_env(key)
        .and_then(|value| value.parse::<usize>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_MAX_NEW_TOKENS" | "XIUXIAN_VISION_OCR_MAX_NEW_TOKENS" => {
                config::max_new_tokens()
            }
            "XIUXIAN_VISION_OCR_CACHE_LOCAL_MAX_ENTRIES" => config::cache_local_max_entries(),
            _ => None,
        })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_u64(key: &str) -> Option<u64> {
    non_empty_env(key)
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_OCR_CACHE_TTL_SECS" => config::cache_ttl_seconds(),
            "XIUXIAN_VISION_OCR_CACHE_TIMEOUT_MS" => config::cache_timeout_ms(),
            _ => None,
        })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_bool(key: &str) -> Option<bool> {
    non_empty_env(key)
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .or_else(|| match key {
            "XIUXIAN_VISION_CROP_MODE" => config::crop_mode(),
            _ => None,
        })
}
