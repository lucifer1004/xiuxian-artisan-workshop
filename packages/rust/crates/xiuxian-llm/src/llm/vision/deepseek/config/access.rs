pub(in crate::llm::vision::deepseek) fn model_root() -> Option<String> {
    super::config().model_root.clone()
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

pub(in crate::llm::vision::deepseek) fn cache_local_max_entries() -> Option<usize> {
    super::config().cache.local_max_entries
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
