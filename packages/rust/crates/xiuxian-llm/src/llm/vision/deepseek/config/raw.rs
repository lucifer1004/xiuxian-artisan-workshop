use serde::Deserialize;

#[xiuxian_macros::xiuxian_config(
    namespace = "llm.vision.deepseek",
    internal_path = "resources/config/vision_deepseek.toml",
    orphan_file = ""
)]
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct DeepseekTomlConfig {
    // Keep fields optional: defaults are sourced from the embedded
    // `resources/config/vision_deepseek.toml` template.
    pub(super) model_root: Option<String>,
    pub(super) model_kind: Option<String>,
    pub(super) client_url: Option<String>,
    pub(super) dots_model_root: Option<String>,
    pub(super) weights_path: Option<String>,
    pub(super) snapshot_path: Option<String>,
    pub(super) ocr_prompt: Option<String>,
    pub(super) device: Option<String>,
    pub(super) base_size: Option<u32>,
    pub(super) image_size: Option<u32>,
    pub(super) crop_mode: Option<bool>,
    pub(super) max_tiles: Option<u32>,
    pub(super) max_new_tokens: Option<usize>,
    pub(super) decode_temperature: Option<f64>,
    pub(super) decode_top_p: Option<f64>,
    pub(super) decode_top_k: Option<usize>,
    pub(super) decode_repetition_penalty: Option<f32>,
    pub(super) decode_use_cache: Option<bool>,
    pub(super) ocr_batch_window_ms: Option<u64>,
    pub(super) ocr_batch_max_size: Option<usize>,
    pub(super) auto_route_complex_min_tiles: Option<u32>,
    pub(super) auto_route_complex_min_pixels: Option<u64>,
    pub(super) ocr_inflight_wait_timeout_ms: Option<u64>,
    pub(super) ocr_inflight_stale_ms: Option<u64>,
    pub(super) cache: DeepseekTomlCacheConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct DeepseekTomlCacheConfig {
    pub(super) local_max_entries: Option<usize>,
    pub(super) preprocess_local_max_entries: Option<usize>,
    pub(super) valkey_url: Option<String>,
    pub(super) key_prefix: Option<String>,
    pub(super) ttl_seconds: Option<u64>,
    pub(super) timeout_ms: Option<u64>,
}
