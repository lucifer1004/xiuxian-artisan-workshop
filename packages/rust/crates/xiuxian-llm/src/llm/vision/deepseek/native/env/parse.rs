use super::super::super::config;
use super::super::super::util::non_empty_env;

pub(in crate::llm::vision::deepseek::native) fn parse_env_string(key: &str) -> Option<String> {
    non_empty_env(key).or_else(|| match key {
        "XIUXIAN_VISION_MODEL_KIND" => config::model_kind(),
        _ => None,
    })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_u32(key: &str) -> Option<u32> {
    non_empty_env(key)
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_BASE_SIZE" => config::base_size(),
            "XIUXIAN_VISION_IMAGE_SIZE" => config::image_size(),
            "XIUXIAN_VISION_MAX_TILES" => config::max_tiles(),
            "XIUXIAN_VISION_AUTO_ROUTE_COMPLEX_MIN_TILES" => config::auto_route_complex_min_tiles(),
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
            "XIUXIAN_VISION_TOP_K" | "XIUXIAN_VISION_OCR_TOP_K" => config::decode_top_k(),
            "XIUXIAN_VISION_OCR_BATCH_MAX_SIZE" => config::ocr_batch_max_size(),
            "XIUXIAN_VISION_OCR_CACHE_LOCAL_MAX_ENTRIES" => config::cache_local_max_entries(),
            _ => None,
        })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_f32(key: &str) -> Option<f32> {
    non_empty_env(key)
        .and_then(|value| value.parse::<f32>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_REPETITION_PENALTY" | "XIUXIAN_VISION_OCR_REPETITION_PENALTY" => {
                config::decode_repetition_penalty()
            }
            _ => None,
        })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_f64(key: &str) -> Option<f64> {
    non_empty_env(key)
        .and_then(|value| value.parse::<f64>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_TEMPERATURE" | "XIUXIAN_VISION_OCR_TEMPERATURE" => {
                config::decode_temperature()
            }
            "XIUXIAN_VISION_TOP_P" | "XIUXIAN_VISION_OCR_TOP_P" => config::decode_top_p(),
            _ => None,
        })
}

pub(in crate::llm::vision::deepseek::native) fn parse_env_u64(key: &str) -> Option<u64> {
    non_empty_env(key)
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_OCR_CACHE_TTL_SECS" => config::cache_ttl_seconds(),
            "XIUXIAN_VISION_OCR_CACHE_TIMEOUT_MS" => config::cache_timeout_ms(),
            "XIUXIAN_VISION_OCR_BATCH_WINDOW_MS" => config::ocr_batch_window_ms(),
            "XIUXIAN_VISION_AUTO_ROUTE_COMPLEX_MIN_PIXELS" => {
                config::auto_route_complex_min_pixels()
            }
            "XIUXIAN_VISION_OCR_INFLIGHT_WAIT_TIMEOUT_MS" => config::ocr_inflight_wait_timeout_ms(),
            "XIUXIAN_VISION_OCR_INFLIGHT_STALE_MS" => config::ocr_inflight_stale_ms(),
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
            "XIUXIAN_VISION_USE_CACHE" | "XIUXIAN_VISION_OCR_USE_CACHE" => {
                config::decode_use_cache()
            }
            _ => None,
        })
}
