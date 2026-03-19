#!/usr/bin/env python3
"""
Runner for real_metal test with both capacity check and runtime memory guard.

1. Uses capfox to check capacity before starting
2. Monitors memory during execution and kills if exceeded
"""

from __future__ import annotations

import os
import pty
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

# ps RSS is ~5x lower than Activity Monitor's Memory on macOS
RSS_SCALE_FACTOR = 5

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CAPFOX_PATH = PROJECT_ROOT / ".run" / "capfox"
VISION_CONFIG_PATH = (
    PROJECT_ROOT
    / "packages"
    / "rust"
    / "crates"
    / "xiuxian-llm"
    / "resources"
    / "config"
    / "vision_deepseek.toml"
)
CPU_TEST_BINARY_PATTERN = "llm_vision_deepseek_real_cpu-"
METAL_TEST_BINARY_PATTERN = "llm_vision_deepseek_real_metal-"
DEFAULT_CPU_MAX_RSS_GB = 12.0
DEFAULT_METAL_MAX_RSS_GB = 10.0
DEFAULT_CUDA_MAX_RSS_GB = DEFAULT_METAL_MAX_RSS_GB
DEFAULT_CPU_CAPFOX_MEM_PERCENT = 50.0
DEFAULT_METAL_CAPFOX_MEM_PERCENT = 30.0
DEFAULT_METAL_CAPFOX_GPU_PERCENT = 80.0
DEFAULT_METAL_CAPFOX_VRAM_PERCENT = 60.0
DEFAULT_CUDA_CAPFOX_MEM_PERCENT = DEFAULT_METAL_CAPFOX_MEM_PERCENT
DEFAULT_CUDA_CAPFOX_GPU_PERCENT = DEFAULT_METAL_CAPFOX_GPU_PERCENT
DEFAULT_CUDA_CAPFOX_VRAM_PERCENT = DEFAULT_METAL_CAPFOX_VRAM_PERCENT


@dataclass(frozen=True)
class TestGuardConfig:
    cpu_max_rss_gb: float = DEFAULT_CPU_MAX_RSS_GB
    metal_max_rss_gb: float = DEFAULT_METAL_MAX_RSS_GB
    cuda_max_rss_gb: float = DEFAULT_CUDA_MAX_RSS_GB
    cpu_capfox_mem_percent: float = DEFAULT_CPU_CAPFOX_MEM_PERCENT
    metal_capfox_mem_percent: float = DEFAULT_METAL_CAPFOX_MEM_PERCENT
    metal_capfox_gpu_percent: float = DEFAULT_METAL_CAPFOX_GPU_PERCENT
    metal_capfox_vram_percent: float = DEFAULT_METAL_CAPFOX_VRAM_PERCENT
    cuda_capfox_mem_percent: float = DEFAULT_CUDA_CAPFOX_MEM_PERCENT
    cuda_capfox_gpu_percent: float = DEFAULT_CUDA_CAPFOX_GPU_PERCENT
    cuda_capfox_vram_percent: float = DEFAULT_CUDA_CAPFOX_VRAM_PERCENT


@dataclass(frozen=True)
class TestProfileConfig:
    max_rss_gb: float | None = None
    capfox_mem_percent: float | None = None
    capfox_gpu_percent: float | None = None
    capfox_vram_percent: float | None = None
    rust_log: str | None = None
    env_overrides: dict[str, str] = field(default_factory=dict)


def _read_text_file(path: Path) -> str | None:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return None
    return text.strip() or None


def _read_test_guard_config(config_path: Path = VISION_CONFIG_PATH) -> TestGuardConfig:
    try:
        payload = tomllib.loads(config_path.read_text(encoding="utf-8"))
    except (FileNotFoundError, tomllib.TOMLDecodeError, OSError):
        return TestGuardConfig()

    section = payload.get("test_guard")
    if not isinstance(section, dict):
        return TestGuardConfig()

    def read_float(key: str, fallback: float) -> float:
        value = section.get(key)
        return float(value) if isinstance(value, int | float) else fallback

    metal_max_rss_gb = read_float("metal_max_rss_gb", DEFAULT_METAL_MAX_RSS_GB)
    metal_capfox_mem_percent = read_float(
        "metal_capfox_mem_percent", DEFAULT_METAL_CAPFOX_MEM_PERCENT
    )
    metal_capfox_gpu_percent = read_float(
        "metal_capfox_gpu_percent", DEFAULT_METAL_CAPFOX_GPU_PERCENT
    )
    metal_capfox_vram_percent = read_float(
        "metal_capfox_vram_percent", DEFAULT_METAL_CAPFOX_VRAM_PERCENT
    )

    return TestGuardConfig(
        cpu_max_rss_gb=read_float("cpu_max_rss_gb", DEFAULT_CPU_MAX_RSS_GB),
        metal_max_rss_gb=metal_max_rss_gb,
        cuda_max_rss_gb=read_float("cuda_max_rss_gb", metal_max_rss_gb),
        cpu_capfox_mem_percent=read_float("cpu_capfox_mem_percent", DEFAULT_CPU_CAPFOX_MEM_PERCENT),
        metal_capfox_mem_percent=metal_capfox_mem_percent,
        metal_capfox_gpu_percent=metal_capfox_gpu_percent,
        metal_capfox_vram_percent=metal_capfox_vram_percent,
        cuda_capfox_mem_percent=read_float("cuda_capfox_mem_percent", metal_capfox_mem_percent),
        cuda_capfox_gpu_percent=read_float("cuda_capfox_gpu_percent", metal_capfox_gpu_percent),
        cuda_capfox_vram_percent=read_float("cuda_capfox_vram_percent", metal_capfox_vram_percent),
    )


def resolve_requested_device(argv: list[str]) -> str:
    requested: str | None = None
    for arg in argv:
        if arg == "--cpu":
            device = "cpu"
        elif arg == "--cuda":
            device = "cuda"
        else:
            continue
        if requested is not None and requested != device:
            raise ValueError("Choose only one of --cpu or --cuda; omit both for Metal.")
        requested = device
    return requested or "metal"


def _read_test_profile_config(
    profile_name: str, config_path: Path = VISION_CONFIG_PATH
) -> TestProfileConfig | None:
    try:
        payload = tomllib.loads(config_path.read_text(encoding="utf-8"))
    except (FileNotFoundError, tomllib.TOMLDecodeError, OSError):
        return None

    profiles = payload.get("test_profiles")
    if not isinstance(profiles, dict):
        return None

    section = profiles.get(profile_name)
    if not isinstance(section, dict):
        return None

    def read_float(key: str) -> float | None:
        value = section.get(key)
        return float(value) if isinstance(value, int | float) else None

    def read_str(key: str) -> str | None:
        value = section.get(key)
        return value.strip() if isinstance(value, str) and value.strip() else None

    def read_bool(key: str) -> bool | None:
        value = section.get(key)
        return value if isinstance(value, bool) else None

    def read_int(key: str) -> int | None:
        value = section.get(key)
        return int(value) if isinstance(value, int) else None

    env_overrides: dict[str, str] = {}
    bool_env_keys = {
        "model_kind": "XIUXIAN_VISION_MODEL_KIND",
        "decode_use_cache": "XIUXIAN_VISION_OCR_USE_CACHE",
        "require_quantized": "XIUXIAN_VISION_REQUIRE_QUANTIZED",
        "allow_empty_output": "XIUXIAN_VISION_ALLOW_EMPTY_OUTPUT",
        "moe_expert_f32_compute": "XIUXIAN_VISION_MOE_EXPERT_F32_COMPUTE",
        "shared_expert_f32_compute": "XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE",
        "skip_shared_experts": "XIUXIAN_VISION_SKIP_SHARED_EXPERTS",
        "stage_trace_stderr": "XIUXIAN_VISION_STAGE_TRACE_STDERR",
        "preload_language_f32_aux": "XIUXIAN_VISION_PRELOAD_LANGUAGE_F32_AUX",
        "preload_vision_f32_aux": "XIUXIAN_VISION_PRELOAD_VISION_F32_AUX",
        "preload_linear_weight_f32": "XIUXIAN_VISION_PRELOAD_LINEAR_WEIGHT_F32",
        "promote_language_input_f32": "XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32",
        "prefill_attention_f32": "XIUXIAN_VISION_PREFILL_ATTENTION_F32",
        "moe_gate_input_f32": "XIUXIAN_VISION_MOE_GATE_INPUT_F32",
        "moe_combine_f32": "XIUXIAN_VISION_MOE_COMBINE_F32",
        "lazy_moe_experts": "XIUXIAN_VISION_LAZY_MOE_EXPERTS",
        "lazy_clip_transformer_layers": "XIUXIAN_VISION_LAZY_CLIP_TRANSFORMER_LAYERS",
    }
    for key, env_key in bool_env_keys.items():
        value = read_bool(key)
        if value is not None:
            env_overrides[env_key] = "1" if value else "0"

    int_env_keys = {
        "base_size": "XIUXIAN_VISION_BASE_SIZE",
        "image_size": "XIUXIAN_VISION_IMAGE_SIZE",
        "max_new_tokens": "XIUXIAN_VISION_OCR_MAX_NEW_TOKENS",
        "min_output_chars": "XIUXIAN_VISION_MIN_OUTPUT_CHARS",
    }
    for key, env_key in int_env_keys.items():
        value = read_int(key)
        if value is not None:
            env_overrides[env_key] = str(value)

    model_kind = read_str("model_kind")
    if model_kind is not None:
        env_overrides["XIUXIAN_VISION_MODEL_KIND"] = model_kind

    moe_backend = read_str("moe_backend")
    if moe_backend is not None:
        env_overrides["XIUXIAN_VISION_MOE_BACKEND"] = moe_backend

    ocr_prompt = read_str("ocr_prompt")
    if ocr_prompt is not None:
        env_overrides["XIUXIAN_VISION_OCR_PROMPT"] = ocr_prompt

    expected_substring = read_str("expected_substring")
    if expected_substring is not None:
        env_overrides["XIUXIAN_VISION_EXPECT_SUBSTRING"] = expected_substring

    crop_mode = read_bool("crop_mode")
    if crop_mode is not None:
        env_overrides["XIUXIAN_VISION_CROP_MODE"] = "1" if crop_mode else "0"

    return TestProfileConfig(
        max_rss_gb=read_float("max_rss_gb"),
        capfox_mem_percent=read_float("capfox_mem_percent"),
        capfox_gpu_percent=read_float("capfox_gpu_percent"),
        capfox_vram_percent=read_float("capfox_vram_percent"),
        rust_log=read_str("rust_log"),
        env_overrides=env_overrides,
    )


def _apply_test_profile(env: dict[str, str], profile: TestProfileConfig) -> None:
    for key, value in profile.env_overrides.items():
        env.setdefault(key, value)


def _format_env_value(value: str) -> str:
    return (
        value.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
    )


def _env_flag_enabled(env: dict[str, str], key: str) -> bool:
    value = env.get(key, "").strip().lower()
    return value in {"1", "true", "yes", "on"}


def _should_use_pty_output(env: dict[str, str]) -> bool:
    return os.name == "posix" and _env_flag_enabled(env, "XIUXIAN_VISION_STAGE_TRACE_STDERR")


def _selected_passthrough_env(
    env: dict[str, str], profile: TestProfileConfig | None
) -> dict[str, str]:
    keys = (
        "XIUXIAN_VISION_MOE_BACKEND",
        "XIUXIAN_VISION_SKIP_SHARED_EXPERTS",
        "XIUXIAN_VISION_STAGE_TRACE_STDERR",
        "DEEPSEEK_OCR_DEBUG_LOGITS_STEP",
        "DEEPSEEK_OCR_DEBUG_LOGITS_JSON",
    )
    profile_keys = set(profile.env_overrides) if profile is not None else set()
    return {
        key: env[key] for key in keys if key in env and key not in profile_keys and env[key].strip()
    }


def _parse_manual_cli_overrides(argv: list[str]) -> tuple[dict[str, str], Path | None]:
    overrides: dict[str, str] = {}
    image_path: Path | None = None
    for arg in argv:
        if arg.startswith("--image="):
            raw = arg.split("=", 1)[1].strip()
            if raw:
                image_path = Path(raw).expanduser()
        elif arg.startswith("--ocr-prompt="):
            raw = arg.split("=", 1)[1]
            if raw.strip():
                overrides["XIUXIAN_VISION_OCR_PROMPT"] = raw
        elif arg.startswith("--ocr-prompt-file="):
            raw = arg.split("=", 1)[1].strip()
            if raw:
                prompt_text = _read_text_file(Path(raw).expanduser())
                if prompt_text is not None:
                    overrides["XIUXIAN_VISION_OCR_PROMPT"] = prompt_text
        elif arg.startswith("--expected-substring="):
            raw = arg.split("=", 1)[1]
            if raw.strip():
                overrides["XIUXIAN_VISION_EXPECT_SUBSTRING"] = raw
        elif arg.startswith("--min-output-chars="):
            raw = arg.split("=", 1)[1].strip()
            if raw:
                overrides["XIUXIAN_VISION_MIN_OUTPUT_CHARS"] = raw
        elif arg.startswith("--max-new-tokens="):
            raw = arg.split("=", 1)[1].strip()
            if raw:
                overrides["XIUXIAN_VISION_OCR_MAX_NEW_TOKENS"] = raw
    return overrides, image_path


def find_test_binary(requested_device: str, use_release: bool) -> Path | None:
    """Find the phase runner test binary for CPU or GPU backends."""
    explicit_binary = os.environ.get("XIUXIAN_VISION_TEST_BINARY")
    if explicit_binary:
        candidate = Path(explicit_binary).expanduser()
        if candidate.exists() and candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
        return None

    profile = "release" if use_release else "debug"
    deps_dir = PROJECT_ROOT / "target" / profile / "deps"
    if not deps_dir.exists():
        return None

    pattern = CPU_TEST_BINARY_PATTERN if requested_device == "cpu" else METAL_TEST_BINARY_PATTERN
    candidates = [
        f
        for f in deps_dir.iterdir()
        if f.name.startswith(pattern)
        and f.is_file()
        and os.access(f, os.X_OK)
        and f.suffix not in (".d", ".o", ".rmeta")
    ]
    if not candidates:
        return None
    candidates.sort(key=lambda path: path.stat().st_mtime, reverse=True)
    return candidates[0]


def get_process_rss_kb(pid: int) -> int:
    """Get RSS in KB using ps command."""
    try:
        result = subprocess.run(
            ["ps", "-o", "rss=", "-p", str(pid)],
            capture_output=True,
            text=True,
            check=False,
        )
        return int(result.stdout.strip()) if result.stdout.strip() else 0
    except (ValueError, OSError):
        return 0


def get_child_processes(ppid: int) -> list[int]:
    """Get all descendant PIDs."""
    try:
        result = subprocess.run(
            ["pgrep", "-P", str(ppid)],
            capture_output=True,
            text=True,
            check=False,
        )
        children = []
        for line in result.stdout.strip().split("\n"):
            if line:
                child_pid = int(line)
                children.append(child_pid)
                children.extend(get_child_processes(child_pid))
        return children
    except (ValueError, OSError):
        return []


def get_total_rss_kb(pid: int) -> int:
    """Get total RSS for process and all descendants."""
    total = get_process_rss_kb(pid)
    for child_pid in get_child_processes(pid):
        total += get_process_rss_kb(child_pid)
    return total


def _read_pty_chunk(master_fd: int) -> str:
    try:
        data = os.read(master_fd, 65536)
    except OSError:
        return ""
    return data.decode("utf-8", errors="replace") if data else ""


def main() -> int:
    # Parse arguments
    use_release = False
    max_rss_gb: float | None = None
    profile_name = os.environ.get("XIUXIAN_VISION_TEST_PROFILE")
    phase = "infer"
    cli_env_overrides, cli_image_path = _parse_manual_cli_overrides(sys.argv[1:])
    try:
        requested_device = resolve_requested_device(sys.argv[1:])
    except ValueError as exc:
        print(f"ERROR: {exc}")
        return 2

    for arg in sys.argv[1:]:
        if arg in {"--cpu", "--cuda"}:
            continue
        elif arg == "--release":
            use_release = True
        elif arg.startswith("--max-rss="):
            max_rss_gb = float(arg.split("=", 1)[1])
        elif arg.startswith("--profile="):
            profile_name = arg.split("=", 1)[1].strip() or None
        elif arg.startswith("--phase="):
            phase = arg.split("=", 1)[1].strip().lower()
        elif (
            arg.startswith("--image=")
            or arg.startswith("--ocr-prompt=")
            or arg.startswith("--ocr-prompt-file=")
            or arg.startswith("--expected-substring=")
            or arg.startswith("--min-output-chars=")
            or arg.startswith("--max-new-tokens=")
        ):
            continue
        elif arg in ("-h", "--help"):
            print(
                f"Usage: {sys.argv[0]} [--cpu|--cuda] [--release] [--phase=load|prewarm|infer] [--profile=NAME] [--max-rss=GB] [--image=PATH] [--ocr-prompt-file=PATH]"
            )
            print("  --cpu         Force CPU device (avoids Metal GPU memory)")
            print(
                "  --cuda        Force CUDA device (reuses Metal-like GPU guard defaults until CUDA-specific guard values are configured)"
            )
            print("  --release     Use target/release test binaries instead of target/debug")
            print("  --phase=...   Run only load, prewarm, or full infer phase (default: infer)")
            print("  --profile=... Apply a TOML-backed test profile from vision_deepseek.toml")
            print("  --max-rss=GB  Maximum RSS in GB (default: 10 for Metal, 12 for CPU)")
            print("  --image=...   Override the default OCR smoke image path")
            print("  --ocr-prompt=... Override OCR prompt directly for a manual probe")
            print("  --ocr-prompt-file=... Read OCR prompt from a file for a manual probe")
            print("  --expected-substring=... Override substring assertion for a manual probe")
            print("  --min-output-chars=... Override minimum output chars for a manual probe")
            print("  --max-new-tokens=... Override decode budget for a manual probe")
            print()
            print("Uses capfox for capacity check, then monitors memory at runtime.")
            return 0

    if phase not in {"load", "prewarm", "infer"}:
        print(f"ERROR: unsupported --phase value: {phase}")
        return 2

    guard = _read_test_guard_config()
    profile = _read_test_profile_config(profile_name) if profile_name is not None else None
    if profile_name is not None and profile is None:
        print(f"ERROR: unknown test profile: {profile_name}")
        return 2
    if max_rss_gb is None:
        if profile and profile.max_rss_gb is not None:
            max_rss_gb = profile.max_rss_gb
        elif requested_device == "cpu":
            max_rss_gb = guard.cpu_max_rss_gb
        elif requested_device == "cuda":
            max_rss_gb = guard.cuda_max_rss_gb
        else:
            max_rss_gb = guard.metal_max_rss_gb

    # Adjust for ps RSS (ps RSS is ~5x lower than Activity Monitor)
    max_rss_kb = int(max_rss_gb * 1024 * 1024 / RSS_SCALE_FACTOR)
    print(f"Max RSS (Activity Monitor): {max_rss_gb} GB")
    print(f"Max RSS (ps, scaled): {max_rss_kb / 1024:.0f} MB")
    print(f"Profile: {'release' if use_release else 'debug'}")
    print(f"Phase: {phase}")
    if profile_name is not None:
        print(f"Config profile: {profile_name}")

    # Find test binary
    binary = find_test_binary(requested_device, use_release)
    if not binary:
        print("ERROR: Test binary not found.")
        print("Run the matching ignored-test binary first.")
        return 1

    print(f"Test binary: {binary}")

    # Check if test image exists
    test_image = cli_image_path or (PROJECT_ROOT / ".run/tmp/ocr-smoke.png")
    if phase == "infer" and not test_image.exists():
        print(f"ERROR: Test image not found: {test_image}")
        return 1

    if phase == "infer":
        print(f"Test image: {test_image}")

    # Check if capfox exists
    capfox = CAPFOX_PATH
    if not capfox.exists():
        print(f"WARNING: capfox not found at {capfox}")
        capfox = None

    # Test expects to be run from xiuxian-llm crate directory for path resolution
    test_cwd = PROJECT_ROOT / "packages" / "rust" / "crates" / "xiuxian-llm"

    env = os.environ.copy()
    env["RUST_BACKTRACE"] = "1"
    if profile is not None:
        _apply_test_profile(env, profile)
    for key, value in cli_env_overrides.items():
        env[key] = value
    env["RUST_LOG"] = os.environ.get(
        "RUST_LOG",
        profile.rust_log if profile and profile.rust_log else "xiuxian_llm=debug,info",
    )
    env["XIUXIAN_VISION_REAL_PHASE"] = phase

    if requested_device == "cpu":
        env["XIUXIAN_VISION_DEVICE"] = "cpu"
        print("Device: CPU (forced)")
    elif requested_device == "cuda":
        env["XIUXIAN_VISION_DEVICE"] = "cuda"
        print("Device: CUDA (forced)")
    else:
        print("Device: Metal/default")
    if profile is not None and profile.env_overrides:
        print("Applied env overrides:")
        for key in sorted(profile.env_overrides):
            resolved = env.get(key, profile.env_overrides[key])
            print(f"  {key}={_format_env_value(resolved)}")
    if cli_env_overrides:
        print("Applied CLI overrides:")
        for key in sorted(cli_env_overrides):
            print(f"  {key}={_format_env_value(cli_env_overrides[key])}")
    passthrough_env = _selected_passthrough_env(env, profile)
    if passthrough_env:
        print("Applied passthrough env:")
        for key in sorted(passthrough_env):
            print(f"  {key}={_format_env_value(passthrough_env[key])}")
    use_pty_output = _should_use_pty_output(env)
    if use_pty_output:
        print("Output transport: PTY")

    # Build test command
    if requested_device == "cpu":
        test_name = "test_real_cpu_inference"
    elif requested_device == "cuda":
        test_name = "test_real_cuda_inference"
    else:
        test_name = "test_real_metal_inference"
    test_cmd = [
        str(binary),
        test_name,
        "--ignored",  # Run ignored tests
        "--test-threads=1",
        "--nocapture",
    ]

    # Phase 1: Capacity check with capfox (fail-open)
    if capfox:
        print()
        print("=== Phase 1: Capacity Check ===")
        if requested_device == "cpu":
            cpu_capfox_mem_percent = (
                profile.capfox_mem_percent
                if profile and profile.capfox_mem_percent is not None
                else guard.cpu_capfox_mem_percent
            )
            check_cmd = [
                str(capfox),
                "run",
                "--task",
                "deepseek_ocr_test",
                "--mem",
                str(cpu_capfox_mem_percent),
                "--reason",
                "--",
                "true",  # Just check capacity, don't run
            ]
        else:
            if requested_device == "cuda":
                gpu_task = "deepseek_ocr_cuda_test"
                gpu_capfox_mem_percent = (
                    profile.capfox_mem_percent
                    if profile and profile.capfox_mem_percent is not None
                    else guard.cuda_capfox_mem_percent
                )
                gpu_capfox_gpu_percent = (
                    profile.capfox_gpu_percent
                    if profile and profile.capfox_gpu_percent is not None
                    else guard.cuda_capfox_gpu_percent
                )
                gpu_capfox_vram_percent = (
                    profile.capfox_vram_percent
                    if profile and profile.capfox_vram_percent is not None
                    else guard.cuda_capfox_vram_percent
                )
            else:
                gpu_task = "deepseek_ocr_metal_test"
                gpu_capfox_mem_percent = (
                    profile.capfox_mem_percent
                    if profile and profile.capfox_mem_percent is not None
                    else guard.metal_capfox_mem_percent
                )
                gpu_capfox_gpu_percent = (
                    profile.capfox_gpu_percent
                    if profile and profile.capfox_gpu_percent is not None
                    else guard.metal_capfox_gpu_percent
                )
                gpu_capfox_vram_percent = (
                    profile.capfox_vram_percent
                    if profile and profile.capfox_vram_percent is not None
                    else guard.metal_capfox_vram_percent
                )
            check_cmd = [
                str(capfox),
                "run",
                "--task",
                gpu_task,
                "--gpu",
                str(gpu_capfox_gpu_percent),
                "--vram",
                str(gpu_capfox_vram_percent),
                "--mem",
                str(gpu_capfox_mem_percent),
                "--reason",
                "--",
                "true",
            ]

        result = subprocess.run(check_cmd, env=env, cwd=test_cwd)
        if result.returncode == 75:
            print("=== CAPACITY DENIED ===")
            print("System does not have capacity for this test.")
            return 75
        # If capfox fails for other reasons, continue anyway (fail-open)

    # Phase 2: Run test with runtime memory guard
    print()
    print("=== Phase 2: Run Test with Memory Guard ===")
    print(f"Starting test (max RSS: {max_rss_gb} GB)...")
    print()

    start_time = time.monotonic()
    master_fd: int | None = None
    if use_pty_output:
        master_fd, slave_fd = pty.openpty()
        try:
            proc = subprocess.Popen(
                test_cmd,
                env=env,
                cwd=test_cwd,
                stdout=slave_fd,
                stderr=slave_fd,
                close_fds=True,
            )
        finally:
            os.close(slave_fd)
    else:
        proc = subprocess.Popen(
            test_cmd,
            env=env,
            cwd=test_cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,  # Merge stderr into stdout
            text=True,
            bufsize=1,  # Line buffered
        )

    killed = False
    try:
        import select

        while proc.poll() is None:
            # Check memory
            total_rss_kb = get_total_rss_kb(proc.pid)
            total_rss_gb = total_rss_kb * RSS_SCALE_FACTOR / 1024 / 1024

            elapsed = time.monotonic() - start_time

            # Read and print output (non-blocking)
            try:
                # Use select to check if there's data to read
                if hasattr(select, "select"):
                    if use_pty_output and master_fd is not None:
                        readable, _, _ = select.select([master_fd], [], [], 0.05)
                        if readable:
                            chunk = _read_pty_chunk(master_fd)
                            if chunk:
                                print(chunk, end="", flush=True)
                    elif proc.stdout is not None:
                        readable, _, _ = select.select([proc.stdout], [], [], 0.05)
                        if readable:
                            line = proc.stdout.readline()
                            if line:
                                print(line, end="", flush=True)
            except:
                pass

            # Print memory status every second
            if int(elapsed) != int(elapsed - 0.1):
                print(f"[{elapsed:.1f}s] RSS: {total_rss_gb:.2f} GB", flush=True)

            # Check memory limit
            if total_rss_kb > max_rss_kb:
                print(f"\n\n*** MEMORY LIMIT EXCEEDED: {total_rss_gb:.2f} GB > {max_rss_gb} GB ***")
                print("*** KILLING PROCESS ***")
                proc.kill()
                killed = True
                break

    except KeyboardInterrupt:
        print("\n*** INTERRUPTED ***")
        proc.kill()
        killed = True

    # Read any remaining output
    if use_pty_output and master_fd is not None:
        while True:
            chunk = _read_pty_chunk(master_fd)
            if not chunk:
                break
            print(chunk, end="", flush=True)
        os.close(master_fd)
    elif proc.stdout:
        remaining = proc.stdout.read()
        if remaining:
            print(remaining, end="", flush=True)

    # Wait for process to finish
    if proc.poll() is None:
        proc.wait()

    exit_code = proc.returncode
    elapsed = time.monotonic() - start_time

    print()
    print(f"Exit code: {exit_code}")
    print(f"Duration: {elapsed:.1f}s")

    if killed:
        print("*** TEST WAS KILLED DUE TO MEMORY LIMIT ***")
        return 137

    if exit_code == 0:
        print("=== TEST PASSED ===")

    return exit_code


if __name__ == "__main__":
    sys.exit(main())
