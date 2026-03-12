#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT_DIR}"

IMAGE_PATH="${1:-}"
if [[ -z ${IMAGE_PATH} ]]; then
  echo "usage: $0 <image-path>" >&2
  exit 2
fi
if [[ ! -f ${IMAGE_PATH} ]]; then
  echo "[ocr-chain] image not found: ${IMAGE_PATH}" >&2
  exit 2
fi

CONFIG_PATH="${OCR_CHAIN_CONFIG_PATH:-${ROOT_DIR}/packages/rust/crates/xiuxian-daochang/resources/config/xiuxian.toml}"
export OCR_CHAIN_ROOT="${ROOT_DIR}"
export OCR_CHAIN_CONFIG_PATH="${CONFIG_PATH}"

python3 - <<'PY'
import os
import sys
from pathlib import Path

try:
    import tomllib  # py>=3.11
except Exception:  # pragma: no cover
    import tomli as tomllib  # type: ignore

root = Path(os.environ["OCR_CHAIN_ROOT"])
config_path = Path(os.environ["OCR_CHAIN_CONFIG_PATH"])
if not config_path.is_file():
    raise SystemExit(f"[ocr-chain] config not found: {config_path}")

data = tomllib.loads(config_path.read_text())
cfg = data.get("llm", {}).get("vision", {}).get("deepseek", {})
model_root = cfg.get("model_root")
snapshot_path = cfg.get("snapshot_path")
if not model_root:
    raise SystemExit("[ocr-chain] llm.vision.deepseek.model_root is required in xiuxian.toml")
if not snapshot_path:
    raise SystemExit("[ocr-chain] llm.vision.deepseek.snapshot_path is required in xiuxian.toml")

model_root_path = Path(model_root)
if not model_root_path.is_absolute():
    model_root_path = (root / model_root_path).resolve()
snapshot_path_obj = Path(snapshot_path)
if not snapshot_path_obj.is_absolute():
    snapshot_path_obj = (root / snapshot_path_obj).resolve()

if not model_root_path.is_dir():
    raise SystemExit(f"[ocr-chain] model_root does not exist: {model_root_path}")
if not snapshot_path_obj.is_file():
    raise SystemExit(f"[ocr-chain] snapshot_path does not exist: {snapshot_path_obj}")

dsq_files = sorted(model_root_path.glob("*.dsq"))
if len(dsq_files) != 1:
    raise SystemExit(
        f"[ocr-chain] expected exactly 1 .dsq under {model_root_path}, found {len(dsq_files)}"
    )
if dsq_files[0].resolve() != snapshot_path_obj:
    raise SystemExit(
        f"[ocr-chain] snapshot_path does not match the only dsq file: {snapshot_path_obj}"
    )

sys.path.insert(0, str(root))
import scripts.fetch_vision_models as fvm  # type: ignore

ok, detail = fvm._scan_dsq_alignment(snapshot_path_obj)
if not ok:
    raise SystemExit(f"[ocr-chain] dsq alignment check failed: {detail}")

print(f"[ocr-chain] config={config_path}")
print(f"[ocr-chain] model_root={model_root_path}")
print(f"[ocr-chain] snapshot_path={snapshot_path_obj}")
print("[ocr-chain] dsq_alignment=PASS")
PY

just probe-ocr-image-guarded "${IMAGE_PATH}"
