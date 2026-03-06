use std::path::Path;

use super::raw::DeepseekTomlConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeepseekCacheConfigSnapshot {
    pub(crate) local_max_entries: Option<usize>,
    pub(crate) preprocess_local_max_entries: Option<usize>,
    pub(crate) valkey_url: Option<String>,
    pub(crate) key_prefix: Option<String>,
    pub(crate) ttl_seconds: Option<u64>,
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeepseekConfigSnapshot {
    pub(crate) model_root: Option<String>,
    pub(crate) model_kind: Option<String>,
    pub(crate) dots_model_root: Option<String>,
    pub(crate) weights_path: Option<String>,
    pub(crate) snapshot_path: Option<String>,
    pub(crate) ocr_prompt: Option<String>,
    pub(crate) device: Option<String>,
    pub(crate) base_size: Option<u32>,
    pub(crate) image_size: Option<u32>,
    pub(crate) crop_mode: Option<bool>,
    pub(crate) max_tiles: Option<u32>,
    pub(crate) max_new_tokens: Option<usize>,
    pub(crate) decode_temperature: Option<f64>,
    pub(crate) decode_top_p: Option<f64>,
    pub(crate) decode_top_k: Option<usize>,
    pub(crate) decode_repetition_penalty: Option<f32>,
    pub(crate) decode_use_cache: Option<bool>,
    pub(crate) ocr_batch_window_ms: Option<u64>,
    pub(crate) ocr_batch_max_size: Option<usize>,
    pub(crate) auto_route_complex_min_tiles: Option<u32>,
    pub(crate) auto_route_complex_min_pixels: Option<u64>,
    pub(crate) ocr_inflight_wait_timeout_ms: Option<u64>,
    pub(crate) ocr_inflight_stale_ms: Option<u64>,
    pub(crate) cache: DeepseekCacheConfigSnapshot,
}

fn to_config_snapshot(config: DeepseekTomlConfig) -> DeepseekConfigSnapshot {
    DeepseekConfigSnapshot {
        model_root: config.model_root,
        model_kind: config.model_kind,
        dots_model_root: config.dots_model_root,
        weights_path: config.weights_path,
        snapshot_path: config.snapshot_path,
        ocr_prompt: config.ocr_prompt,
        device: config.device,
        base_size: config.base_size,
        image_size: config.image_size,
        crop_mode: config.crop_mode,
        max_tiles: config.max_tiles,
        max_new_tokens: config.max_new_tokens,
        decode_temperature: config.decode_temperature,
        decode_top_p: config.decode_top_p,
        decode_top_k: config.decode_top_k,
        decode_repetition_penalty: config.decode_repetition_penalty,
        decode_use_cache: config.decode_use_cache,
        ocr_batch_window_ms: config.ocr_batch_window_ms,
        ocr_batch_max_size: config.ocr_batch_max_size,
        auto_route_complex_min_tiles: config.auto_route_complex_min_tiles,
        auto_route_complex_min_pixels: config.auto_route_complex_min_pixels,
        ocr_inflight_wait_timeout_ms: config.ocr_inflight_wait_timeout_ms,
        ocr_inflight_stale_ms: config.ocr_inflight_stale_ms,
        cache: DeepseekCacheConfigSnapshot {
            local_max_entries: config.cache.local_max_entries,
            preprocess_local_max_entries: config.cache.preprocess_local_max_entries,
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
