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
    cache_valkey_url, client_url, crop_mode, device, dots_model_root, image_size, max_new_tokens,
    model_kind, model_root, ocr_prompt, preprocess_local_max_entries, snapshot_path, weights_path,
};
pub(crate) use self::snapshot::{DeepseekConfigSnapshot, load_config_with_paths_for_tests};
