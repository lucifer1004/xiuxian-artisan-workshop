#!/usr/bin/env bash
set -euo pipefail

resolved_model_root="${1:-}"
if [ -z "${resolved_model_root}" ]; then
  if [ -d ".data/models/dots-ocr" ]; then
    resolved_model_root=".data/models/dots-ocr"
  elif [ -n "${PRJ_DATA_HOME:-}" ] && [ -d "${PRJ_DATA_HOME}/models/dots-ocr" ]; then
    resolved_model_root="${PRJ_DATA_HOME}/models/dots-ocr"
  else
    echo "Error: dots-ocr model directory not found. Run: just fetch-vision-models" >&2
    exit 1
  fi
fi

default_device="cpu"
case "$(uname -s)" in
Darwin)
  default_device="metal"
  ;;
Linux)
  default_device="cuda"
  ;;
esac

export XIUXIAN_VISION_MODEL_KIND="${XIUXIAN_VISION_MODEL_KIND:-dots}"
export XIUXIAN_VISION_MODEL_PATH="${XIUXIAN_VISION_MODEL_PATH:-${resolved_model_root}}"
export XIUXIAN_VISION_DEVICE="${XIUXIAN_VISION_DEVICE:-${default_device}}"
export XIUXIAN_VISION_REQUIRE_QUANTIZED="${XIUXIAN_VISION_REQUIRE_QUANTIZED:-0}"
export XIUXIAN_VISION_OCR_MAX_NEW_TOKENS="${XIUXIAN_VISION_OCR_MAX_NEW_TOKENS:-1024}"
export XIUXIAN_VISION_MAX_TILES="${XIUXIAN_VISION_MAX_TILES:-12}"

echo "[vision-heavy] model_root=${XIUXIAN_VISION_MODEL_PATH}"
echo "[vision-heavy] device=${XIUXIAN_VISION_DEVICE}"

NEXTTEST_GUARD_LABEL="${NEXTTEST_GUARD_LABEL:-vision-heavy}" \
  NEXTTEST_GUARD_MAX_RSS_GB="${NEXTTEST_GUARD_MAX_RSS_GB:-24}" \
  just nextest-guarded \
  -p xiuxian-llm \
  --release \
  --test llm_vision_deepseek_smoke \
  --test-threads "${VISION_HEAVY_TEST_THREADS:-1}" \
  deepseek_smoke_runs_real_inference_from_local_model_cache
