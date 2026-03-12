use std::path::Path;

use super::raw::DeepseekTomlConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeepseekCacheConfigSnapshot {
    pub(crate) local_max_entries: Option<usize>,
    pub(crate) valkey_url: Option<String>,
    pub(crate) key_prefix: Option<String>,
    pub(crate) ttl_seconds: Option<u64>,
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeepseekConfigSnapshot {
    pub(crate) model_root: Option<String>,
    pub(crate) weights_path: Option<String>,
    pub(crate) snapshot_path: Option<String>,
    pub(crate) ocr_prompt: Option<String>,
    pub(crate) device: Option<String>,
    pub(crate) base_size: Option<u32>,
    pub(crate) image_size: Option<u32>,
    pub(crate) crop_mode: Option<bool>,
    pub(crate) max_new_tokens: Option<usize>,
    pub(crate) cache: DeepseekCacheConfigSnapshot,
}

fn to_config_snapshot(config: DeepseekTomlConfig) -> DeepseekConfigSnapshot {
    DeepseekConfigSnapshot {
        model_root: config.model_root,
        weights_path: config.weights_path,
        snapshot_path: config.snapshot_path,
        ocr_prompt: config.ocr_prompt,
        device: config.device,
        base_size: config.base_size,
        image_size: config.image_size,
        crop_mode: config.crop_mode,
        max_new_tokens: config.max_new_tokens,
        cache: DeepseekCacheConfigSnapshot {
            local_max_entries: config.cache.local_max_entries,
            valkey_url: config.cache.valkey_url,
            key_prefix: config.cache.key_prefix,
            ttl_seconds: config.cache.ttl_seconds,
            timeout_ms: config.cache.timeout_ms,
        },
    }
}

pub(crate) fn load_config_with_paths_for_tests(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> DeepseekConfigSnapshot {
    let config =
        DeepseekTomlConfig::load_with_paths(project_root, config_home).unwrap_or_else(|error| {
            panic!("failed to load deepseek config in test with explicit paths: {error}")
        });
    to_config_snapshot(config)
}
