//! `DeepSeek` runtime model-root resolver tests.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use xiuxian_llm::llm::acceleration::AccelerationDevice;
use xiuxian_llm::test_support::{
    deepseek_snapshot_qoffset_aligned_for_tests, normalize_deepseek_model_root,
    require_quantized_deepseek_snapshot_for_tests, resolve_deepseek_device_kind_label_for_tests,
    resolve_deepseek_model_kind_for_model_root_label_for_tests,
    resolve_deepseek_model_kind_label_for_tests, resolve_deepseek_model_root_with,
    should_retry_deepseek_cpu_fallback_for_tests,
};

#[test]
fn resolve_model_root_with_prefers_env_value() {
    let resolved = resolve_deepseek_model_root_with(
        Some("/models/from-env"),
        Some("/models/from-config"),
        Some("/models/from-default"),
    );
    assert_eq!(resolved.as_deref(), Some("/models/from-env"));
}

#[test]
fn resolve_model_root_with_falls_back_to_config() {
    let resolved = resolve_deepseek_model_root_with(
        None,
        Some("/models/from-config"),
        Some("/models/from-default"),
    );
    assert_eq!(resolved.as_deref(), Some("/models/from-config"));
}

#[test]
fn resolve_model_root_with_falls_back_to_default() {
    let resolved = resolve_deepseek_model_root_with(None, None, Some("/models/from-default"));
    assert_eq!(resolved.as_deref(), Some("/models/from-default"));
}

#[test]
fn resolve_model_root_with_returns_none_when_all_sources_absent() {
    let resolved = resolve_deepseek_model_root_with(None, None, None);
    assert_eq!(resolved, None);
}

#[test]
fn normalize_model_root_keeps_absolute_path() {
    let project_root = Path::new("/repo/root");
    let resolved = normalize_deepseek_model_root("/abs/models/deepseek-ocr-2", project_root);
    assert_eq!(resolved, "/abs/models/deepseek-ocr-2");
}

#[test]
fn normalize_model_root_resolves_relative_to_project_root() {
    let project_root = Path::new("/repo/root");
    let resolved = normalize_deepseek_model_root(".data/models/deepseek-ocr-2", project_root);
    assert_eq!(resolved, "/repo/root/.data/models/deepseek-ocr-2");
}

#[test]
fn resolve_device_kind_explicit_metal_without_device_falls_back_to_cpu() {
    let label = resolve_deepseek_device_kind_label_for_tests(
        Some("metal"),
        AccelerationDevice::Auto,
        false,
    );
    assert_eq!(label, "cpu");
}

#[test]
fn resolve_device_kind_explicit_unknown_falls_back_to_cpu() {
    let label = resolve_deepseek_device_kind_label_for_tests(
        Some("unknown"),
        AccelerationDevice::Cuda,
        true,
    );
    assert_eq!(label, "cpu");
}

#[test]
fn resolve_device_kind_without_explicit_uses_fallback_mode() {
    let label = resolve_deepseek_device_kind_label_for_tests(None, AccelerationDevice::Cpu, true);
    assert_eq!(label, "cpu");
}

#[test]
fn resolve_device_kind_explicit_metal_with_device_selects_expected_backend() {
    let label =
        resolve_deepseek_device_kind_label_for_tests(Some("metal"), AccelerationDevice::Cpu, true);

    #[cfg(feature = "vision-dots-metal")]
    assert_eq!(label, "metal");
    #[cfg(not(feature = "vision-dots-metal"))]
    assert_eq!(label, "cpu");
}

#[test]
fn cpu_fallback_retry_detects_metal_buffer_allocation_error() {
    let should_retry = should_retry_deepseek_cpu_fallback_for_tests(
        "deepseek OCR decode failed: Metal error Failed to create metal resource: Buffer",
    );
    assert!(should_retry);
}

#[test]
fn cpu_fallback_retry_ignores_unrelated_decode_errors() {
    let should_retry = should_retry_deepseek_cpu_fallback_for_tests(
        "deepseek OCR decode failed: unsupported Qwen2 query length 8500",
    );
    assert!(!should_retry);
}

#[test]
fn require_quantized_snapshot_defaults_to_enabled() {
    assert!(require_quantized_deepseek_snapshot_for_tests(None));
}

#[test]
fn require_quantized_snapshot_parses_explicit_false_values() {
    for value in ["0", "false", "no", "off"] {
        assert!(!require_quantized_deepseek_snapshot_for_tests(Some(value)));
    }
}

#[test]
fn require_quantized_snapshot_treats_other_values_as_enabled() {
    for value in ["1", "true", "yes", "on", "anything"] {
        assert!(require_quantized_deepseek_snapshot_for_tests(Some(value)));
    }
}

#[test]
fn snapshot_qoffset_alignment_requires_even_offsets_for_q4k() {
    // DsqTensorDType::Q4K => code 12
    assert!(deepseek_snapshot_qoffset_aligned_for_tests(2, 12));
    assert!(!deepseek_snapshot_qoffset_aligned_for_tests(3, 12));
}

#[test]
fn snapshot_qoffset_alignment_requires_four_byte_alignment_for_f32() {
    // DsqTensorDType::F32 => code 0
    assert!(deepseek_snapshot_qoffset_aligned_for_tests(8, 0));
    assert!(!deepseek_snapshot_qoffset_aligned_for_tests(6, 0));
}

#[test]
fn snapshot_qoffset_alignment_rejects_unknown_dtype_codes() {
    assert!(!deepseek_snapshot_qoffset_aligned_for_tests(8, 999));
}

#[test]
fn resolve_model_kind_label_normalizes_supported_aliases() {
    assert_eq!(
        resolve_deepseek_model_kind_label_for_tests(Some("deepseek-ocr")),
        "deepseek"
    );
    assert_eq!(
        resolve_deepseek_model_kind_label_for_tests(Some("paddleocr_vl")),
        "paddle_ocr_vl"
    );
    assert_eq!(
        resolve_deepseek_model_kind_label_for_tests(Some("vl2")),
        "dots_ocr"
    );
}

#[test]
fn resolve_model_kind_label_falls_back_to_dots_for_unknown_values() {
    assert_eq!(
        resolve_deepseek_model_kind_label_for_tests(Some("unknown-backend")),
        "dots_ocr"
    );
}

#[test]
fn resolve_model_kind_for_model_root_promotes_dots_layout() {
    let mut root = env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    root.push(format!(
        "xiuxian-llm-deepseek-model-kind-root-{}-{nanos}",
        std::process::id()
    ));
    if let Err(error) = fs::create_dir_all(&root) {
        panic!(
            "failed to create temporary model root {}: {error}",
            root.display()
        );
    }
    if let Err(error) = fs::write(root.join("model.safetensors.index.json"), "{}") {
        panic!(
            "failed to write model index marker under {}: {error}",
            root.display()
        );
    }

    let resolved = resolve_deepseek_model_kind_for_model_root_label_for_tests(
        Some("deepseek"),
        root.as_path(),
    );
    assert_eq!(resolved, "dots_ocr");

    let _ = fs::remove_dir_all(root);
}
