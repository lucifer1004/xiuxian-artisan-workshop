pub(in crate::llm::vision::deepseek) fn model_root() -> Option<String> {
    super::config().model_root.clone()
}

pub(in crate::llm::vision::deepseek) fn model_kind() -> Option<String> {
    super::config().model_kind.clone()
}

pub(in crate::llm::vision::deepseek) fn client_url() -> Option<String> {
    super::config().client_url.clone()
}

pub(in crate::llm::vision::deepseek) fn dots_model_root() -> Option<String> {
    super::config().dots_model_root.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn weights_path() -> Option<String> {
    super::config().weights_path.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn snapshot_path() -> Option<String> {
    super::config().snapshot_path.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn ocr_prompt() -> Option<String> {
    super::config().ocr_prompt.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn device() -> Option<String> {
    super::config().device.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn require_quantized() -> Option<bool> {
    super::config().require_quantized
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn base_size() -> Option<u32> {
    super::config().base_size
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn image_size() -> Option<u32> {
    super::config().image_size
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn crop_mode() -> Option<bool> {
    super::config().crop_mode
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn max_new_tokens() -> Option<usize> {
    super::config().max_new_tokens
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn max_tiles() -> Option<u32> {
    super::config().max_tiles
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn decode_temperature() -> Option<f64> {
    super::config().decode_temperature
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn decode_top_p() -> Option<f64> {
    super::config().decode_top_p
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn decode_top_k() -> Option<usize> {
    super::config().decode_top_k
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn decode_repetition_penalty() -> Option<f32> {
    super::config().decode_repetition_penalty
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn decode_use_cache() -> Option<bool> {
    super::config().decode_use_cache
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn preload_language_f32_aux() -> Option<bool> {
    super::config().preload_language_f32_aux
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn preload_vision_f32_aux() -> Option<bool> {
    super::config().preload_vision_f32_aux
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn preload_linear_weight_f32() -> Option<bool> {
    super::config().preload_linear_weight_f32
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn promote_language_input_f32() -> Option<bool> {
    super::config().promote_language_input_f32
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn lazy_moe_experts() -> Option<bool> {
    super::config().lazy_moe_experts
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn lazy_clip_transformer_layers() -> Option<bool> {
    super::config().lazy_clip_transformer_layers
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn cache_local_max_entries() -> Option<usize> {
    super::config().cache.local_max_entries
}

pub(in crate::llm::vision::deepseek) fn preprocess_local_max_entries() -> Option<usize> {
    super::config().cache.preprocess_local_max_entries
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn cache_valkey_url() -> Option<String> {
    super::config().cache.valkey_url.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn cache_key_prefix() -> Option<String> {
    super::config().cache.key_prefix.clone()
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn cache_ttl_seconds() -> Option<u64> {
    super::config().cache.ttl_seconds
}

#[cfg(feature = "vision-dots")]
pub(in crate::llm::vision::deepseek) fn cache_timeout_ms() -> Option<u64> {
    super::config().cache.timeout_ms
}
