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
    base_size, cache_key_prefix, cache_local_max_entries, cache_timeout_ms, cache_ttl_seconds,
    cache_valkey_url, client_url, crop_mode, decode_repetition_penalty, decode_temperature,
    decode_top_k, decode_top_p, decode_use_cache, device, dots_model_root, image_size,
    lazy_clip_transformer_layers, lazy_moe_experts, max_new_tokens, max_tiles, model_kind,
    model_root, ocr_prompt, preload_language_f32_aux, preload_linear_weight_f32,
    preload_vision_f32_aux, preprocess_local_max_entries, promote_language_input_f32,
    require_quantized, snapshot_path, weights_path,
};
pub(crate) use self::snapshot::{DeepseekConfigSnapshot, load_config_with_paths_for_tests};
