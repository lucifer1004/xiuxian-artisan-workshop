mod access;
mod loader;
mod raw;
mod snapshot;

use std::sync::OnceLock;

use self::raw::DeepseekTomlConfig;

static CONFIG: OnceLock<DeepseekTomlConfig> = OnceLock::new();

fn config() -> &'static DeepseekTomlConfig {
    CONFIG.get_or_init(loader::load_config)
}

pub(super) use self::access::{
    auto_route_complex_min_pixels, auto_route_complex_min_tiles, base_size, cache_key_prefix,
    cache_local_max_entries, cache_timeout_ms, cache_ttl_seconds, cache_valkey_url, crop_mode,
    decode_repetition_penalty, decode_temperature, decode_top_k, decode_top_p, decode_use_cache,
    device, dots_model_root, image_size, max_new_tokens, max_tiles, model_kind, model_root,
    ocr_batch_max_size, ocr_batch_window_ms, ocr_inflight_stale_ms, ocr_inflight_wait_timeout_ms,
    ocr_prompt, preprocess_local_max_entries, snapshot_path, weights_path,
};
pub(crate) use self::snapshot::{DeepseekConfigSnapshot, load_config_with_paths_for_tests};
