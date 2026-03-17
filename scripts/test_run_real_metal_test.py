from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_MODULE_PATH = Path(__file__).resolve().with_name("run_real_metal_test.py")
_SPEC = importlib.util.spec_from_file_location("run_real_metal_test", _MODULE_PATH)
assert _SPEC and _SPEC.loader
_MODULE = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = _MODULE
_SPEC.loader.exec_module(_MODULE)


def test_read_test_guard_config_uses_toml_values(tmp_path: Path) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_guard]
cpu_max_rss_gb = 14.0
metal_max_rss_gb = 16.0
cpu_capfox_mem_percent = 55.0
metal_capfox_mem_percent = 35.0
metal_capfox_gpu_percent = 85.0
metal_capfox_vram_percent = 65.0
""".strip()
        + "\n",
        encoding="utf-8",
    )

    guard = _MODULE._read_test_guard_config(config_path)

    assert guard.cpu_max_rss_gb == 14.0
    assert guard.metal_max_rss_gb == 16.0
    assert guard.cpu_capfox_mem_percent == 55.0
    assert guard.metal_capfox_mem_percent == 35.0
    assert guard.metal_capfox_gpu_percent == 85.0
    assert guard.metal_capfox_vram_percent == 65.0


def test_read_test_guard_config_falls_back_when_missing() -> None:
    guard = _MODULE._read_test_guard_config(Path("/nonexistent/vision_deepseek.toml"))

    assert guard == _MODULE.TestGuardConfig()


def test_read_test_profile_config_reads_env_overrides(tmp_path: Path) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_guard_12g]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 448
image_size = 448
crop_mode = true
max_new_tokens = 32
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config("deepseek_metal_guard_12g", config_path)

    assert profile is not None
    assert profile.max_rss_gb == 12.0
    assert profile.capfox_mem_percent == 30.0
    assert profile.capfox_gpu_percent == 80.0
    assert profile.capfox_vram_percent == 60.0
    assert profile.rust_log == "info"
    assert profile.env_overrides == {
        "XIUXIAN_VISION_BASE_SIZE": "448",
        "XIUXIAN_VISION_CROP_MODE": "1",
        "XIUXIAN_VISION_IMAGE_SIZE": "448",
        "XIUXIAN_VISION_OCR_MAX_NEW_TOKENS": "32",
        "XIUXIAN_VISION_OCR_USE_CACHE": "0",
        "XIUXIAN_VISION_LAZY_CLIP_TRANSFORMER_LAYERS": "1",
        "XIUXIAN_VISION_LAZY_MOE_EXPERTS": "1",
        "XIUXIAN_VISION_MODEL_KIND": "deepseek",
        "XIUXIAN_VISION_PRELOAD_LANGUAGE_F32_AUX": "0",
        "XIUXIAN_VISION_PRELOAD_LINEAR_WEIGHT_F32": "0",
        "XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32": "0",
        "XIUXIAN_VISION_PRELOAD_VISION_F32_AUX": "0",
        "XIUXIAN_VISION_REQUIRE_QUANTIZED": "0",
    }


def test_apply_test_profile_respects_existing_env() -> None:
    profile = _MODULE.TestProfileConfig(
        env_overrides={
            "XIUXIAN_VISION_MODEL_KIND": "deepseek",
            "XIUXIAN_VISION_LAZY_MOE_EXPERTS": "1",
        }
    )
    env = {
        "XIUXIAN_VISION_MODEL_KIND": "dots",
    }

    _MODULE._apply_test_profile(env, profile)

    assert env["XIUXIAN_VISION_MODEL_KIND"] == "dots"
    assert env["XIUXIAN_VISION_LAZY_MOE_EXPERTS"] == "1"


def test_read_test_profile_config_supports_safe384_smoke_profile(tmp_path: Path) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
allow_empty_output = true
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config("deepseek_metal_smoke_12g_safe384", config_path)

    assert profile is not None
    assert profile.max_rss_gb == 12.0
    assert profile.env_overrides == {
        "XIUXIAN_VISION_BASE_SIZE": "384",
        "XIUXIAN_VISION_CROP_MODE": "0",
        "XIUXIAN_VISION_IMAGE_SIZE": "384",
        "XIUXIAN_VISION_ALLOW_EMPTY_OUTPUT": "1",
        "XIUXIAN_VISION_OCR_MAX_NEW_TOKENS": "1",
        "XIUXIAN_VISION_OCR_USE_CACHE": "0",
        "XIUXIAN_VISION_LAZY_CLIP_TRANSFORMER_LAYERS": "1",
        "XIUXIAN_VISION_LAZY_MOE_EXPERTS": "1",
        "XIUXIAN_VISION_MODEL_KIND": "deepseek",
        "XIUXIAN_VISION_PRELOAD_LANGUAGE_F32_AUX": "0",
        "XIUXIAN_VISION_PRELOAD_LINEAR_WEIGHT_F32": "0",
        "XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32": "0",
        "XIUXIAN_VISION_PRELOAD_VISION_F32_AUX": "0",
        "XIUXIAN_VISION_REQUIRE_QUANTIZED": "0",
    }


def test_read_test_profile_config_supports_multiline_prompt_override(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1", config_path
    )

    assert profile is not None
    assert (
        profile.env_overrides["XIUXIAN_VISION_OCR_PROMPT"]
        == "<image>\n<|grounding|>Return exactly one visible digit from the image."
    )


def test_format_env_value_escapes_newlines() -> None:
    assert _MODULE._format_env_value("<image>\nline\t2") == "<image>\\nline\\t2"


def test_env_flag_enabled_parses_truthy_values() -> None:
    env = {"XIUXIAN_VISION_STAGE_TRACE_STDERR": "yes"}

    assert _MODULE._env_flag_enabled(env, "XIUXIAN_VISION_STAGE_TRACE_STDERR") is True


def test_should_use_pty_output_tracks_stage_trace_flag() -> None:
    env = {"XIUXIAN_VISION_STAGE_TRACE_STDERR": "1"}

    assert _MODULE._should_use_pty_output(env) == (_MODULE.os.name == "posix")
    assert _MODULE._should_use_pty_output({}) is False


def test_read_test_profile_config_supports_moe_expert_compute_toggle(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_safe320_digit1_native_moe]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 320
image_size = 320
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
moe_expert_f32_compute = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_safe320_digit1_native_moe", config_path
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_MOE_EXPERT_F32_COMPUTE"] == "0"


def test_read_test_profile_config_supports_language_input_dtype_toggle(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1_native_inputs]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1_native_inputs", config_path
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32"] == "0"


def test_read_test_profile_config_supports_prefill_attention_dtype_toggle(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
prefill_attention_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn",
        config_path,
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_PREFILL_ATTENTION_F32"] == "0"


def test_read_test_profile_config_supports_moe_combine_dtype_toggle(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_combine]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
prefill_attention_f32 = false
moe_combine_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_combine",
        config_path,
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_MOE_COMBINE_F32"] == "0"


def test_read_test_profile_config_supports_moe_gate_input_dtype_toggle(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
prefill_attention_f32 = false
moe_gate_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs",
        config_path,
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_MOE_GATE_INPUT_F32"] == "0"


def test_read_test_profile_config_supports_min_output_chars_override(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
min_output_chars = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
lazy_moe_experts = true
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1",
        config_path,
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_MIN_OUTPUT_CHARS"] == "1"


def test_read_test_profile_config_supports_metal_fast_shared_diagnostics(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs_metal_fast_eager_skip_shared]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
min_output_chars = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
prefill_attention_f32 = false
moe_gate_input_f32 = false
moe_backend = "metal_fast"
skip_shared_experts = true
stage_trace_stderr = true
lazy_moe_experts = false
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs_metal_fast_eager_skip_shared",
        config_path,
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_MOE_BACKEND"] == "metal_fast"
    assert profile.env_overrides["XIUXIAN_VISION_SKIP_SHARED_EXPERTS"] == "1"
    assert profile.env_overrides["XIUXIAN_VISION_STAGE_TRACE_STDERR"] == "1"
    assert profile.env_overrides["XIUXIAN_VISION_LAZY_MOE_EXPERTS"] == "0"


def test_read_test_profile_config_supports_shared_expert_compute_toggle(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "vision_deepseek.toml"
    config_path.write_text(
        """
[test_profiles.deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs_metal_fast_eager_shared_native]
max_rss_gb = 12.0
capfox_mem_percent = 30.0
capfox_gpu_percent = 80.0
capfox_vram_percent = 60.0
rust_log = "info"
model_kind = "deepseek"
base_size = 384
image_size = 384
crop_mode = false
max_new_tokens = 1
min_output_chars = 1
decode_use_cache = false
require_quantized = false
preload_language_f32_aux = false
preload_vision_f32_aux = false
preload_linear_weight_f32 = false
promote_language_input_f32 = false
prefill_attention_f32 = false
moe_gate_input_f32 = false
moe_backend = "metal_fast"
shared_expert_f32_compute = false
lazy_moe_experts = false
lazy_clip_transformer_layers = true
ocr_prompt = "<image>\\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
""".strip()
        + "\n",
        encoding="utf-8",
    )

    profile = _MODULE._read_test_profile_config(
        "deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs_metal_fast_eager_shared_native",
        config_path,
    )

    assert profile is not None
    assert profile.env_overrides["XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE"] == "0"


def test_selected_passthrough_env_reports_non_profile_overrides() -> None:
    env = {
        "XIUXIAN_VISION_MOE_BACKEND": "metal_fast",
        "XIUXIAN_VISION_SKIP_SHARED_EXPERTS": "1",
        "XIUXIAN_VISION_STAGE_TRACE_STDERR": "1",
        "XIUXIAN_VISION_MODEL_KIND": "deepseek",
    }
    profile = _MODULE.TestProfileConfig(env_overrides={"XIUXIAN_VISION_MODEL_KIND": "deepseek"})

    passthrough = _MODULE._selected_passthrough_env(env, profile)

    assert passthrough == {
        "XIUXIAN_VISION_MOE_BACKEND": "metal_fast",
        "XIUXIAN_VISION_SKIP_SHARED_EXPERTS": "1",
        "XIUXIAN_VISION_STAGE_TRACE_STDERR": "1",
    }


def test_find_test_binary_prefers_explicit_override(monkeypatch, tmp_path: Path) -> None:
    binary = tmp_path / "llm_vision_deepseek_real_metal-explicit"
    binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    binary.chmod(0o755)
    monkeypatch.setenv("XIUXIAN_VISION_TEST_BINARY", str(binary))

    found = _MODULE.find_test_binary(use_cpu=False, use_release=False)

    assert found == binary


def test_find_test_binary_rejects_missing_explicit_override(monkeypatch) -> None:
    monkeypatch.setenv("XIUXIAN_VISION_TEST_BINARY", "/nonexistent/llm_vision")

    assert _MODULE.find_test_binary(use_cpu=False, use_release=False) is None
