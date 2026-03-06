pub(in crate::llm::vision::deepseek) fn model_root() -> Option<String> {
    super::config().model_root.clone()
}

pub(in crate::llm::vision::deepseek) fn model_kind() -> Option<String> {
    super::config().model_kind.clone()
}

pub(in crate::llm::vision::deepseek) fn dots_model_root() -> Option<String> {
    super::config().dots_model_root.clone()
}

pub(in crate::llm::vision::deepseek) fn weights_path() -> Option<String> {
    super::config().weights_path.clone()
}

pub(in crate::llm::vision::deepseek) fn snapshot_path() -> Option<String> {
    super::config().snapshot_path.clone()
}

pub(in crate::llm::vision::deepseek) fn ocr_prompt() -> Option<String> {
    super::config().ocr_prompt.clone()
}

pub(in crate::llm::vision::deepseek) fn device() -> Option<String> {
    super::config().device.clone()
}

pub(in crate::llm::vision::deepseek) fn base_size() -> Option<u32> {
    super::config().base_size
}

pub(in crate::llm::vision::deepseek) fn image_size() -> Option<u32> {
    super::config().image_size
}

pub(in crate::llm::vision::deepseek) fn crop_mode() -> Option<bool> {
    super::config().crop_mode
}

pub(in crate::llm::vision::deepseek) fn max_tiles() -> Option<u32> {
    super::config().max_tiles
}

pub(in crate::llm::vision::deepseek) fn max_new_tokens() -> Option<usize> {
    super::config().max_new_tokens
}

pub(in crate::llm::vision::deepseek) fn decode_temperature() -> Option<f64> {
    super::config().decode_temperature
}

pub(in crate::llm::vision::deepseek) fn decode_top_p() -> Option<f64> {
    super::config().decode_top_p
}

pub(in crate::llm::vision::deepseek) fn decode_top_k() -> Option<usize> {
    super::config().decode_top_k
}

pub(in crate::llm::vision::deepseek) fn decode_repetition_penalty() -> Option<f32> {
    super::config().decode_repetition_penalty
}

pub(in crate::llm::vision::deepseek) fn decode_use_cache() -> Option<bool> {
    super::config().decode_use_cache
}

pub(in crate::llm::vision::deepseek) fn ocr_batch_window_ms() -> Option<u64> {
    super::config().ocr_batch_window_ms
}

pub(in crate::llm::vision::deepseek) fn ocr_batch_max_size() -> Option<usize> {
    super::config().ocr_batch_max_size
}

pub(in crate::llm::vision::deepseek) fn auto_route_complex_min_tiles() -> Option<u32> {
    super::config().auto_route_complex_min_tiles
}

pub(in crate::llm::vision::deepseek) fn auto_route_complex_min_pixels() -> Option<u64> {
    super::config().auto_route_complex_min_pixels
}

pub(in crate::llm::vision::deepseek) fn ocr_inflight_wait_timeout_ms() -> Option<u64> {
    super::config().ocr_inflight_wait_timeout_ms
}

pub(in crate::llm::vision::deepseek) fn ocr_inflight_stale_ms() -> Option<u64> {
    super::config().ocr_inflight_stale_ms
}

pub(in crate::llm::vision::deepseek) fn cache_local_max_entries() -> Option<usize> {
    super::config().cache.local_max_entries
}

pub(in crate::llm::vision::deepseek) fn preprocess_local_max_entries() -> Option<usize> {
    super::config().cache.preprocess_local_max_entries
}

pub(in crate::llm::vision::deepseek) fn cache_valkey_url() -> Option<String> {
    super::config().cache.valkey_url.clone()
}

pub(in crate::llm::vision::deepseek) fn cache_key_prefix() -> Option<String> {
    super::config().cache.key_prefix.clone()
}

pub(in crate::llm::vision::deepseek) fn cache_ttl_seconds() -> Option<u64> {
    super::config().cache.ttl_seconds
}

pub(in crate::llm::vision::deepseek) fn cache_timeout_ms() -> Option<u64> {
    super::config().cache.timeout_ms
}
