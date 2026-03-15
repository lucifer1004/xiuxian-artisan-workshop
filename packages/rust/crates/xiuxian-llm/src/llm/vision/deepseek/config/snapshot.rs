use std::path::Path;

use super::raw::DeepseekTomlConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepseekCacheConfigSnapshot {
    pub local_max_entries: Option<usize>,
    pub valkey_url: Option<String>,
    pub key_prefix: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepseekConfigSnapshot {
    pub model_root: Option<String>,
    pub weights_path: Option<String>,
    pub snapshot_path: Option<String>,
    pub ocr_prompt: Option<String>,
    pub device: Option<String>,
    pub base_size: Option<u32>,
    pub image_size: Option<u32>,
    pub crop_mode: Option<bool>,
    pub max_new_tokens: Option<usize>,
    pub cache: DeepseekCacheConfigSnapshot,
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
