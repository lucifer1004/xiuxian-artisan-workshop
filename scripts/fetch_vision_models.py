#!/usr/bin/env python3
"""Fetch Dots OCR model artifacts for xiuxian-llm vision integration."""

from __future__ import annotations

import argparse
import os
import shutil
import struct
from pathlib import Path

DEFAULT_REPO_ID = "rednote-hilab/dots.ocr"
DEFAULT_MODEL_DIR = "dots-ocr"
LEGACY_MODEL_DIRS = ("deepseek-ocr", "deepseek-ocr-2", "paddleocr-vl")
DEFAULT_QUANTIZATION = "auto"


def _project_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _prj_data_home(project_root: Path) -> Path:
    configured = os.environ.get("PRJ_DATA_HOME", "").strip()
    if configured:
        path = Path(configured)
        return path if path.is_absolute() else project_root / path
    return project_root / ".data"


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Fetch Dots OCR model artifacts for xiuxian-llm vision integration."
    )
    parser.add_argument(
        "--repo-id",
        default=None,
        help=f"Hugging Face repo id override (default: {DEFAULT_REPO_ID})",
    )
    parser.add_argument(
        "--model-dir",
        default=None,
        help=f"model directory under $PRJ_DATA_HOME/models (default: {DEFAULT_MODEL_DIR})",
    )
    parser.add_argument(
        "--revision",
        default=None,
        help="optional Hugging Face revision/tag/commit",
    )
    parser.add_argument(
        "--quantization",
        choices=["auto", "q4k", "q6k", "q8_0", "none"],
        default=DEFAULT_QUANTIZATION,
        help="quantized snapshot preference (default: auto)",
    )
    parser.add_argument(
        "--prune-legacy",
        action=argparse.BooleanOptionalAction,
        default=True,
        help=(
            "remove legacy OCR model directories under $PRJ_DATA_HOME/models "
            f"({', '.join(LEGACY_MODEL_DIRS)}) after success (default: true)"
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print resolved paths without downloading",
    )
    return parser


def _prune_legacy_model_dirs(prj_data_home: Path, target_dir: Path) -> None:
    for legacy_dir_name in LEGACY_MODEL_DIRS:
        legacy_dir = prj_data_home / "models" / legacy_dir_name
        if legacy_dir == target_dir:
            continue
        if not legacy_dir.exists():
            print(f"Legacy directory not found: {legacy_dir}")
            continue
        shutil.rmtree(legacy_dir)
        print(f"Removed legacy directory: {legacy_dir}")


def _read_u32(handle) -> int:
    return struct.unpack("<I", handle.read(4))[0]


def _read_u64(handle) -> int:
    return struct.unpack("<Q", handle.read(8))[0]


def _read_string(handle) -> str:
    length = _read_u32(handle)
    raw = handle.read(length)
    return raw.decode("utf-8", errors="replace")


def _scan_dsq_alignment(path: Path) -> tuple[bool, str]:
    alignment_by_dtype = {
        0: 4,  # F32
        1: 2,  # F16
        8: 2,  # Q8_0
        12: 2,  # Q4K
        14: 2,  # Q6K
        16: 2,  # BF16
    }
    try:
        with path.open("rb") as handle:
            magic = handle.read(7)
            if magic != b"DSQSNAP":
                return False, "invalid magic"
            _version = _read_u32(handle)
            _candle_version = _read_string(handle)
            _model_id = _read_string(handle)
            _backend = _read_string(handle)
            _default_dtype = _read_u32(handle)
            _block_size = _read_u32(handle)
            tensor_count = _read_u32(handle)
            violations: list[str] = []
            for _ in range(tensor_count):
                name = _read_string(handle)
                _out_dim = _read_u32(handle)
                _in_dim = _read_u32(handle)
                q_dtype = _read_u32(handle)
                q_offset = _read_u64(handle)
                _q_len = _read_u64(handle)
                _bias_offset = _read_u64(handle)
                _bias_len = _read_u64(handle)
                _bias_dtype = _read_u32(handle)
                required = alignment_by_dtype.get(q_dtype)
                if required and q_offset % required != 0:
                    violations.append(
                        f"{name}: q_offset={q_offset}, dtype_code={q_dtype}, required_align={required}"
                    )
                    if len(violations) >= 3:
                        break
            if violations:
                return False, "; ".join(violations)
        return True, "ok"
    except Exception as error:
        return False, str(error)


def _build_allow_patterns(quantization: str) -> list[str]:
    patterns = [
        "config.json",
        "preprocessor_config.json",
        "tokenizer.json",
        "*.safetensors",
        "*.index.json",
        "*.txt",
    ]
    patterns.extend(_build_dsq_allow_patterns(quantization))
    return patterns


def _build_dsq_allow_patterns(quantization: str) -> list[str]:
    if quantization == "none":
        return []
    if quantization == "auto":
        return ["*.dsq"]

    tokens_by_quantization = {
        "q4k": ("q4k", "q4_k"),
        "q6k": ("q6k", "q6_k"),
        "q8_0": ("q8_0", "q8-0", "q80", "q8"),
    }
    tokens = tokens_by_quantization.get(quantization, ())
    patterns = {f"*{token}*.dsq" for token in tokens}
    patterns.update(f"*{token.upper()}*.dsq" for token in tokens)
    return sorted(patterns)


def _dsq_matches_quantization(path: Path, quantization: str) -> bool:
    if quantization in {"auto", "none"}:
        return True
    lower = path.name.lower()
    if quantization == "q4k":
        return "q4k" in lower or "q4_k" in lower
    if quantization == "q6k":
        return "q6k" in lower or "q6_k" in lower
    if quantization == "q8_0":
        return any(token in lower for token in ("q8_0", "q8-0", "q80", "q8"))
    return False


def fetch_models(args: argparse.Namespace) -> int:
    project_root = _project_root()
    prj_data_home = _prj_data_home(project_root)
    repo_id = args.repo_id or DEFAULT_REPO_ID
    model_dir = args.model_dir or DEFAULT_MODEL_DIR
    target_dir = prj_data_home / "models" / model_dir
    allow_patterns = _build_allow_patterns(args.quantization)
    print("\n" + "=" * 80)
    print(f"Downloading OCR artifacts to: {target_dir}")
    print("Model profile: dots-only")
    print(f"Repo: {repo_id}")
    if args.revision:
        print(f"Revision: {args.revision}")
    print(f"Quantization: {args.quantization}")
    print("=" * 80)

    if args.dry_run:
        print("Dry-run mode enabled. No files downloaded.")
        return 0

    target_dir.mkdir(parents=True, exist_ok=True)

    try:
        from huggingface_hub import snapshot_download

        path = snapshot_download(
            repo_id=repo_id,
            local_dir=target_dir,
            revision=args.revision,
            allow_patterns=allow_patterns,
            ignore_patterns=["*.msgpack", "*.h5"],
        )
        print(f"\nModel download completed: {path}")
        dsq_files = sorted(target_dir.rglob("*.dsq"))
        if args.quantization == "none":
            print("DSQ download disabled by --quantization=none.")
        elif dsq_files:
            print(f"Detected {len(dsq_files)} DSQ snapshot file(s).")
            matched = [
                path for path in dsq_files if _dsq_matches_quantization(path, args.quantization)
            ]
            if args.quantization != "auto" and not matched:
                print(
                    "Warning: DSQ files were downloaded, but none matched requested quantization "
                    f"{args.quantization}."
                )
            for dsq_file in dsq_files:
                print(f"  - {dsq_file}")
                valid, detail = _scan_dsq_alignment(dsq_file)
                if valid:
                    print("    alignment-check: PASS")
                else:
                    print("    alignment-check: FAIL")
                    print(f"    detail: {detail}")
                    print(
                        "    note: this DSQ may abort candle quantized loading due to unaligned offsets."
                    )
        else:
            print(
                "No DSQ snapshot files were downloaded. "
                "Set --quantization=none and XIUXIAN_VISION_REQUIRE_QUANTIZED=0 for pure safetensors mode."
            )
        if args.prune_legacy:
            _prune_legacy_model_dirs(prj_data_home, target_dir)
        print("Set this environment variable before running vision OCR:")
        print(f'  export XIUXIAN_VISION_MODEL_PATH="{target_dir}"')
        print('  export XIUXIAN_VISION_MODEL_KIND="dots"')
        print("=" * 80)
        return 0
    except Exception as error:
        print(f"\nModel download failed: {error}")
        return 1


def main() -> int:
    parser = _build_parser()
    return fetch_models(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
