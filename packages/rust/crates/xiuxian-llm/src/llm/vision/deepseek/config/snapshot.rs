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

#[derive(Debug, Clone, PartialEq)]
pub struct DeepseekTestGuardConfigSnapshot {
    pub cpu_max_rss_gb: Option<f64>,
    pub metal_max_rss_gb: Option<f64>,
    pub cpu_capfox_mem_percent: Option<f64>,
    pub metal_capfox_mem_percent: Option<f64>,
    pub metal_capfox_gpu_percent: Option<f64>,
    pub metal_capfox_vram_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeepseekConfigSnapshot {
    pub model_root: Option<String>,
    pub model_kind: Option<String>,
    pub dots_model_root: Option<String>,
    pub weights_path: Option<String>,
    pub snapshot_path: Option<String>,
    pub ocr_prompt: Option<String>,
    pub device: Option<String>,
    pub require_quantized: Option<bool>,
    pub base_size: Option<u32>,
    pub image_size: Option<u32>,
    pub crop_mode: Option<bool>,
    pub max_tiles: Option<u32>,
    pub max_new_tokens: Option<usize>,
    pub decode_temperature: Option<f64>,
    pub decode_top_p: Option<f64>,
    pub decode_top_k: Option<usize>,
    pub decode_repetition_penalty: Option<f32>,
    pub decode_use_cache: Option<bool>,
    pub preload_language_f32_aux: Option<bool>,
    pub preload_vision_f32_aux: Option<bool>,
    pub preload_linear_weight_f32: Option<bool>,
    pub promote_language_input_f32: Option<bool>,
    pub lazy_moe_experts: Option<bool>,
    pub lazy_clip_transformer_layers: Option<bool>,
    pub test_guard: DeepseekTestGuardConfigSnapshot,
    pub cache: DeepseekCacheConfigSnapshot,
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
        require_quantized: config.require_quantized,
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
        preload_language_f32_aux: config.preload_language_f32_aux,
        preload_vision_f32_aux: config.preload_vision_f32_aux,
        preload_linear_weight_f32: config.preload_linear_weight_f32,
        promote_language_input_f32: config.promote_language_input_f32,
        lazy_moe_experts: config.lazy_moe_experts,
        lazy_clip_transformer_layers: config.lazy_clip_transformer_layers,
        test_guard: DeepseekTestGuardConfigSnapshot {
            cpu_max_rss_gb: config.test_guard.cpu_max_rss_gb,
            metal_max_rss_gb: config.test_guard.metal_max_rss_gb,
            cpu_capfox_mem_percent: config.test_guard.cpu_capfox_mem_percent,
            metal_capfox_mem_percent: config.test_guard.metal_capfox_mem_percent,
            metal_capfox_gpu_percent: config.test_guard.metal_capfox_gpu_percent,
            metal_capfox_vram_percent: config.test_guard.metal_capfox_vram_percent,
        },
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
