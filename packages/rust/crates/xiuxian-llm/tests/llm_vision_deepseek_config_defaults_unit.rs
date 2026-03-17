//! `DeepSeek` OCR embedded-config parity tests.

use tempfile::tempdir;
use xiuxian_llm::test_support::{
    load_deepseek_config_with_paths, resolve_default_deepseek_model_root_with,
};

#[test]
fn embedded_deepseek_defaults_match_upstream_baseline() {
    let project_root = tempdir().expect("project root tempdir");
    let config_home = tempdir().expect("config home tempdir");
    let snapshot =
        load_deepseek_config_with_paths(Some(project_root.path()), Some(config_home.path()));

    assert_eq!(snapshot.model_root, None);
    assert_eq!(snapshot.model_kind.as_deref(), Some("deepseek"));
    assert_eq!(
        snapshot.dots_model_root.as_deref(),
        Some(".data/models/dots-ocr")
    );
    assert_eq!(snapshot.require_quantized, Some(false));
    assert_eq!(snapshot.base_size, Some(1_024));
    assert_eq!(snapshot.image_size, Some(640));
    assert_eq!(snapshot.crop_mode, Some(true));
    assert_eq!(snapshot.max_tiles, Some(12));
    assert_eq!(snapshot.max_new_tokens, Some(512));
    assert_eq!(snapshot.decode_temperature, Some(0.0));
    assert_eq!(snapshot.decode_repetition_penalty, Some(1.0));
    assert_eq!(snapshot.decode_use_cache, Some(true));
    assert_eq!(snapshot.preload_language_f32_aux, Some(true));
    assert_eq!(snapshot.preload_vision_f32_aux, Some(true));
    assert_eq!(snapshot.preload_linear_weight_f32, Some(true));
    assert_eq!(snapshot.promote_language_input_f32, Some(true));
    assert_eq!(snapshot.lazy_moe_experts, Some(false));
    assert_eq!(snapshot.lazy_clip_transformer_layers, Some(false));
    assert_eq!(snapshot.test_guard.cpu_max_rss_gb, Some(12.0));
    assert_eq!(snapshot.test_guard.metal_max_rss_gb, Some(12.0));
    assert_eq!(snapshot.test_guard.cpu_capfox_mem_percent, Some(50.0));
    assert_eq!(snapshot.test_guard.metal_capfox_mem_percent, Some(30.0));
    assert_eq!(snapshot.test_guard.metal_capfox_gpu_percent, Some(80.0));
    assert_eq!(snapshot.test_guard.metal_capfox_vram_percent, Some(60.0));
}

#[test]
fn default_model_root_prefers_deepseek_ocr_before_ocr2() {
    let cache_home = tempdir().expect("cache home tempdir");
    let data_home = tempdir().expect("data home tempdir");

    let preferred = cache_home.path().join("models/deepseek-ocr");
    let fallback = data_home.path().join("models/deepseek-ocr-2");
    std::fs::create_dir_all(&preferred).expect("create preferred model dir");
    std::fs::create_dir_all(&fallback).expect("create fallback model dir");

    let resolved = resolve_default_deepseek_model_root_with(cache_home.path(), data_home.path());
    assert_eq!(
        resolved.as_deref(),
        Some(preferred.to_string_lossy().as_ref())
    );
}
