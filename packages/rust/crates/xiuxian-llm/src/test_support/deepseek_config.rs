use std::path::Path;

/// Test-facing `DeepSeek` cache configuration snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepseekCacheConfigSnapshot {
    /// Maximum local cache entries.
    pub local_max_entries: Option<usize>,
    /// Optional `valkey` endpoint URL.
    pub valkey_url: Option<String>,
    /// Optional key prefix for shared caches.
    pub key_prefix: Option<String>,
    /// Optional cache TTL in seconds.
    pub ttl_seconds: Option<u64>,
    /// Optional cache operation timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Test-facing real-test guard configuration snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepseekTestGuardConfigSnapshot {
    /// Default CPU RSS limit in Activity Monitor GB.
    pub cpu_max_rss_gb: Option<f64>,
    /// Default Metal RSS limit in Activity Monitor GB.
    pub metal_max_rss_gb: Option<f64>,
    /// Capfox CPU-side memory admission percentage.
    pub cpu_capfox_mem_percent: Option<f64>,
    /// Capfox Metal-side memory admission percentage.
    pub metal_capfox_mem_percent: Option<f64>,
    /// Capfox Metal-side GPU admission percentage.
    pub metal_capfox_gpu_percent: Option<f64>,
    /// Capfox Metal-side VRAM admission percentage.
    pub metal_capfox_vram_percent: Option<f64>,
}

/// Test-facing `DeepSeek` runtime configuration snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepseekConfigSnapshot {
    /// Model root directory.
    pub model_root: Option<String>,
    /// Explicit model kind hint.
    pub model_kind: Option<String>,
    /// DotsOCR-specific model root override.
    pub dots_model_root: Option<String>,
    /// Weights path override.
    pub weights_path: Option<String>,
    /// Snapshot path override.
    pub snapshot_path: Option<String>,
    /// OCR prompt template override.
    pub ocr_prompt: Option<String>,
    /// Device hint.
    pub device: Option<String>,
    /// Whether a quantized snapshot is required.
    pub require_quantized: Option<bool>,
    /// Base size used by preprocessing.
    pub base_size: Option<u32>,
    /// Image size used by preprocessing.
    pub image_size: Option<u32>,
    /// Crop-mode switch.
    pub crop_mode: Option<bool>,
    /// Maximum number of image tiles to process.
    pub max_tiles: Option<u32>,
    /// Decoder max new tokens.
    pub max_new_tokens: Option<usize>,
    /// Decoder temperature.
    pub decode_temperature: Option<f64>,
    /// Decoder top-p sampling bound.
    pub decode_top_p: Option<f64>,
    /// Decoder top-k sampling bound.
    pub decode_top_k: Option<usize>,
    /// Decoder repetition penalty.
    pub decode_repetition_penalty: Option<f32>,
    /// Decoder cache toggle.
    pub decode_use_cache: Option<bool>,
    /// Whether to preload F32 language output auxiliaries for low-precision loads.
    pub preload_language_f32_aux: Option<bool>,
    /// Whether to preload F32 projector and vision auxiliaries for low-precision loads.
    pub preload_vision_f32_aux: Option<bool>,
    /// Whether to preload F32 copies for linear layer weights in low-precision loads.
    pub preload_linear_weight_f32: Option<bool>,
    /// Whether language input embeddings should be promoted to F32 before decode.
    pub promote_language_input_f32: Option<bool>,
    /// Whether `MoE` experts should be materialized lazily at first use.
    pub lazy_moe_experts: Option<bool>,
    /// Whether CLIP transformer layers should be materialized lazily at first use.
    pub lazy_clip_transformer_layers: Option<bool>,
    /// Script/test guard defaults.
    pub test_guard: DeepseekTestGuardConfigSnapshot,
    /// Cache subsection snapshot.
    pub cache: DeepseekCacheConfigSnapshot,
}

/// Load `DeepSeek` config with explicit path roots for integration tests.
#[must_use]
pub fn load_deepseek_config_with_paths(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> DeepseekConfigSnapshot {
    let snapshot =
        crate::llm::vision::deepseek::load_config_with_paths_for_tests(project_root, config_home);
    DeepseekConfigSnapshot {
        model_root: snapshot.model_root,
        model_kind: snapshot.model_kind,
        dots_model_root: snapshot.dots_model_root,
        weights_path: snapshot.weights_path,
        snapshot_path: snapshot.snapshot_path,
        ocr_prompt: snapshot.ocr_prompt,
        device: snapshot.device,
        require_quantized: snapshot.require_quantized,
        base_size: snapshot.base_size,
        image_size: snapshot.image_size,
        crop_mode: snapshot.crop_mode,
        max_tiles: snapshot.max_tiles,
        max_new_tokens: snapshot.max_new_tokens,
        decode_temperature: snapshot.decode_temperature,
        decode_top_p: snapshot.decode_top_p,
        decode_top_k: snapshot.decode_top_k,
        decode_repetition_penalty: snapshot.decode_repetition_penalty,
        decode_use_cache: snapshot.decode_use_cache,
        preload_language_f32_aux: snapshot.preload_language_f32_aux,
        preload_vision_f32_aux: snapshot.preload_vision_f32_aux,
        preload_linear_weight_f32: snapshot.preload_linear_weight_f32,
        promote_language_input_f32: snapshot.promote_language_input_f32,
        lazy_moe_experts: snapshot.lazy_moe_experts,
        lazy_clip_transformer_layers: snapshot.lazy_clip_transformer_layers,
        test_guard: DeepseekTestGuardConfigSnapshot {
            cpu_max_rss_gb: snapshot.test_guard.cpu_max_rss_gb,
            metal_max_rss_gb: snapshot.test_guard.metal_max_rss_gb,
            cpu_capfox_mem_percent: snapshot.test_guard.cpu_capfox_mem_percent,
            metal_capfox_mem_percent: snapshot.test_guard.metal_capfox_mem_percent,
            metal_capfox_gpu_percent: snapshot.test_guard.metal_capfox_gpu_percent,
            metal_capfox_vram_percent: snapshot.test_guard.metal_capfox_vram_percent,
        },
        cache: DeepseekCacheConfigSnapshot {
            local_max_entries: snapshot.cache.local_max_entries,
            valkey_url: snapshot.cache.valkey_url,
            key_prefix: snapshot.cache.key_prefix,
            ttl_seconds: snapshot.cache.ttl_seconds,
            timeout_ms: snapshot.cache.timeout_ms,
        },
    }
}

/// Resolve the `DeepSeek` model root using env/config/default precedence.
#[must_use]
pub fn resolve_deepseek_model_root_with(
    env_model_root: Option<&str>,
    config_model_root: Option<&str>,
    default_model_root: Option<&str>,
) -> Option<String> {
    crate::llm::vision::deepseek::resolve_model_root_with_for_tests(
        env_model_root,
        config_model_root,
        default_model_root,
    )
}

/// Resolve the default `DeepSeek` model root search path using explicit cache/data homes.
#[must_use]
pub fn resolve_default_deepseek_model_root_with(
    cache_home: &Path,
    data_home: &Path,
) -> Option<String> {
    crate::llm::vision::deepseek::resolve_default_model_root_with_for_tests(cache_home, data_home)
}

/// Normalize `DeepSeek` model root relative to `project_root` when needed.
#[must_use]
pub fn normalize_deepseek_model_root(raw: &str, project_root: &Path) -> String {
    crate::llm::vision::deepseek::normalize_model_root_for_tests(raw, project_root)
}
