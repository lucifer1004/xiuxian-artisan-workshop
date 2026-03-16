//! `DeepSeek` OCR loader policy unit tests.

use std::fs;

use tempfile::tempdir;
use xiuxian_llm::test_support::{
    resolve_deepseek_load_dtype_label_for_tests,
    resolve_deepseek_model_kind_for_model_root_label_for_tests,
    resolve_deepseek_safe_vision_settings_for_tests, resolve_deepseek_vision_settings_for_tests,
};

#[test]
fn accelerated_backend_defaults_to_f16() {
    assert_eq!(
        resolve_deepseek_load_dtype_label_for_tests(None, true),
        "f16"
    );
}

#[test]
fn cpu_backend_defaults_to_f32() {
    assert_eq!(
        resolve_deepseek_load_dtype_label_for_tests(None, false),
        "f32"
    );
}

#[test]
fn explicit_dtype_is_preserved() {
    assert_eq!(
        resolve_deepseek_load_dtype_label_for_tests(Some("bf16"), true),
        "bf16"
    );
}

#[test]
fn default_vision_settings_match_upstream_cli() {
    assert_eq!(
        resolve_deepseek_vision_settings_for_tests(None, None, None),
        (1_024, 640, true)
    );
}

#[test]
fn explicit_vision_overrides_are_preserved() {
    assert_eq!(
        resolve_deepseek_vision_settings_for_tests(Some(896), Some(512), Some(false)),
        (896, 512, false)
    );
}

#[test]
fn safe_fallback_vision_settings_stay_ocr2_compatible() {
    assert_eq!(
        resolve_deepseek_safe_vision_settings_for_tests(),
        (448, 448, true)
    );
}

#[test]
fn explicit_deepseek_model_kind_is_not_overridden_by_dots_index_file() {
    let tempdir = tempdir().expect("tempdir");
    let root = tempdir.path();
    fs::write(root.join("model.safetensors.index.json"), "{}").expect("write index");

    assert_eq!(
        resolve_deepseek_model_kind_for_model_root_label_for_tests(Some("deepseek"), root),
        "deepseek"
    );
}

#[test]
fn auto_detection_still_falls_back_to_dots_for_dots_like_roots() {
    let tempdir = tempdir().expect("tempdir");
    let root = tempdir.path();
    fs::write(root.join("model.safetensors.index.json"), "{}").expect("write index");

    assert_eq!(
        resolve_deepseek_model_kind_for_model_root_label_for_tests(None, root),
        "dots_ocr"
    );
}
