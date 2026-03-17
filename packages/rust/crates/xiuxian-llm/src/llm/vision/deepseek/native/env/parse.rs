use crate::llm::vision::deepseek::config;
use crate::llm::vision::deepseek::util::non_empty_env;

pub(in crate::llm::vision::deepseek::native) fn parse_env_u32(key: &str) -> Option<u32> {
    non_empty_env(key)
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| match key {
            "XIUXIAN_VISION_BASE_SIZE" => config::base_size(),
            "XIUXIAN_VISION_IMAGE_SIZE" => config::image_size(),
            "XIUXIAN_VISION_MAX_TILES" => config::max_tiles(),
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
            "XIUXIAN_VISION_REQUIRE_QUANTIZED" => config::require_quantized(),
            "XIUXIAN_VISION_USE_CACHE" | "XIUXIAN_VISION_OCR_USE_CACHE" => {
                config::decode_use_cache()
            }
            "XIUXIAN_VISION_PRELOAD_LANGUAGE_F32_AUX" => config::preload_language_f32_aux(),
            "XIUXIAN_VISION_PRELOAD_VISION_F32_AUX" => config::preload_vision_f32_aux(),
            "XIUXIAN_VISION_PRELOAD_LINEAR_WEIGHT_F32" => config::preload_linear_weight_f32(),
            "XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32" => config::promote_language_input_f32(),
            "XIUXIAN_VISION_LAZY_MOE_EXPERTS" => config::lazy_moe_experts(),
            "XIUXIAN_VISION_LAZY_CLIP_TRANSFORMER_LAYERS" => config::lazy_clip_transformer_layers(),
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

pub(in crate::llm::vision::deepseek::native) fn parse_env_string(key: &str) -> Option<String> {
    non_empty_env(key)
}
