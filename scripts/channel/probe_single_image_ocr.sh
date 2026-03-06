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
  echo "[ocr-probe] image not found: ${IMAGE_PATH}" >&2
  exit 2
fi

export XIUXIAN_OCR_PROBE_IMAGE="${IMAGE_PATH}"
export XIUXIAN_OCR_PROBE_CARGO_PROFILE="${XIUXIAN_OCR_PROBE_CARGO_PROFILE:-dev}"
export XIUXIAN_OCR_PROBE_PREBUILD="${XIUXIAN_OCR_PROBE_PREBUILD:-0}"
export XIUXIAN_VISION_OCR_TIMEOUT_MS="${XIUXIAN_VISION_OCR_TIMEOUT_MS:-120000}"
export XIUXIAN_VISION_OCR_COLD_TIMEOUT_MS="${XIUXIAN_VISION_OCR_COLD_TIMEOUT_MS:-120000}"
export XIUXIAN_VISION_OCR_PREWARM_TIMEOUT_MS="${XIUXIAN_VISION_OCR_PREWARM_TIMEOUT_MS:-120000}"
export XIUXIAN_VISION_OCR_MAX_DIMENSION="${XIUXIAN_VISION_OCR_MAX_DIMENSION:-1024}"
export XIUXIAN_VISION_OCR_MAX_NEW_TOKENS="${XIUXIAN_VISION_OCR_MAX_NEW_TOKENS:-1024}"
export XIUXIAN_VISION_OCR_PREFER_ORIGINAL="${XIUXIAN_VISION_OCR_PREFER_ORIGINAL:-1}"
export XIUXIAN_VISION_OCR_ORIGINAL_MAX_BYTES="${XIUXIAN_VISION_OCR_ORIGINAL_MAX_BYTES:-16777216}"
export XIUXIAN_VISION_OCR_FAILURE_COOLDOWN_MS="${XIUXIAN_VISION_OCR_FAILURE_COOLDOWN_MS:-15000}"
export XIUXIAN_VISION_OCR_HEARTBEAT_MS="${XIUXIAN_VISION_OCR_HEARTBEAT_MS:-5000}"
export XIUXIAN_OCR_PROBE_HEARTBEAT_SECS="${XIUXIAN_OCR_PROBE_HEARTBEAT_SECS:-5}"
export XIUXIAN_OCR_PROBE_HARD_TIMEOUT_SECS="${XIUXIAN_OCR_PROBE_HARD_TIMEOUT_SECS:-120}"
export RUST_LOG="${RUST_LOG:-xiuxian_daochang::llm::compat::litellm_ocr=debug,xiuxian_llm::llm::vision=info}"
if [[ -n ${XIUXIAN_VISION_DEVICE:-} ]]; then
  export XIUXIAN_VISION_DEVICE
fi

echo "[ocr-probe] image=${XIUXIAN_OCR_PROBE_IMAGE}"
echo "[ocr-probe] cargo_profile=${XIUXIAN_OCR_PROBE_CARGO_PROFILE}"
echo "[ocr-probe] prebuild=${XIUXIAN_OCR_PROBE_PREBUILD}"
echo "[ocr-probe] timeout_ms=${XIUXIAN_VISION_OCR_TIMEOUT_MS} cold_timeout_ms=${XIUXIAN_VISION_OCR_COLD_TIMEOUT_MS}"
echo "[ocr-probe] prewarm_timeout_ms=${XIUXIAN_VISION_OCR_PREWARM_TIMEOUT_MS}"
echo "[ocr-probe] max_dimension=${XIUXIAN_VISION_OCR_MAX_DIMENSION} max_new_tokens=${XIUXIAN_VISION_OCR_MAX_NEW_TOKENS}"
echo "[ocr-probe] prefer_original=${XIUXIAN_VISION_OCR_PREFER_ORIGINAL} original_max_bytes=${XIUXIAN_VISION_OCR_ORIGINAL_MAX_BYTES}"
echo "[ocr-probe] failure_cooldown_ms=${XIUXIAN_VISION_OCR_FAILURE_COOLDOWN_MS}"
echo "[ocr-probe] worker_heartbeat_ms=${XIUXIAN_VISION_OCR_HEARTBEAT_MS} probe_heartbeat_secs=${XIUXIAN_OCR_PROBE_HEARTBEAT_SECS}"
echo "[ocr-probe] vision_device=${XIUXIAN_VISION_DEVICE:-<from-config>}"
echo "[ocr-probe] hard_timeout_secs=${XIUXIAN_OCR_PROBE_HARD_TIMEOUT_SECS}"
echo "[ocr-probe] rust_log=${RUST_LOG}"

if command -v gtimeout >/dev/null 2>&1; then
  TIMEOUT_BIN="gtimeout"
elif command -v timeout >/dev/null 2>&1; then
  TIMEOUT_BIN="timeout"
else
  TIMEOUT_BIN=""
fi

resolve_profile_dir() {
  local profile="$1"
  case "${profile}" in
  dev | debug)
    printf '%s' "debug"
    ;;
  *)
    printf '%s' "${profile}"
    ;;
  esac
}

if [[ ${XIUXIAN_OCR_PROBE_PREBUILD} == "1" ]]; then
  echo "[ocr-probe] prebuilding test binary (outside hard timeout window)"
  if [[ ${XIUXIAN_OCR_PROBE_CARGO_PROFILE} == "release" ]]; then
    cargo test -p xiuxian-daochang --test llm --release --no-run
  elif [[ ${XIUXIAN_OCR_PROBE_CARGO_PROFILE} == "dev" || ${XIUXIAN_OCR_PROBE_CARGO_PROFILE} == "debug" ]]; then
    cargo test -p xiuxian-daochang --test llm --no-run
  else
    cargo test \
      -p xiuxian-daochang \
      --test llm \
      --profile "${XIUXIAN_OCR_PROBE_CARGO_PROFILE}" \
      --no-run
  fi
fi

TARGET_DIR="${CARGO_TARGET_DIR:-target}"
PROFILE_DIR="${TARGET_DIR}/$(resolve_profile_dir "${XIUXIAN_OCR_PROBE_CARGO_PROFILE}")/deps"
if [[ ! -d ${PROFILE_DIR} ]]; then
  echo "[ocr-probe] profile dir not found: ${PROFILE_DIR}" >&2
  echo "[ocr-probe] hint: set XIUXIAN_OCR_PROBE_PREBUILD=1 for first run" >&2
  exit 2
fi

shopt -s nullglob
CANDIDATES=("${PROFILE_DIR}"/llm-*)
EXECUTABLES=()
for candidate in "${CANDIDATES[@]}"; do
  if [[ -f ${candidate} && -x ${candidate} ]]; then
    EXECUTABLES+=("${candidate}")
  fi
done
shopt -u nullglob

if [[ ${#EXECUTABLES[@]} -eq 0 ]]; then
  echo "[ocr-probe] no executable llm test binary under ${PROFILE_DIR}" >&2
  echo "[ocr-probe] hint: set XIUXIAN_OCR_PROBE_PREBUILD=1 for first run" >&2
  exit 2
fi

PROBE_BIN="$(ls -1t "${EXECUTABLES[@]}" | head -n 1)"
if [[ -z ${PROBE_BIN} ]]; then
  echo "[ocr-probe] failed to select llm test binary" >&2
  exit 2
fi
echo "[ocr-probe] probe_bin=${PROBE_BIN}"

export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"
RUN_TEST_CMD=(
  "${PROBE_BIN}"
  --ignored
  --exact
  litellm_ocr_probe_tests::litellm_ocr_probe_single_image
  --nocapture
)

RUNNER_CMD=("${RUN_TEST_CMD[@]}")
if [[ -n ${TIMEOUT_BIN} ]]; then
  RUNNER_CMD=(
    "${TIMEOUT_BIN}" --signal=TERM --kill-after=20s "${XIUXIAN_OCR_PROBE_HARD_TIMEOUT_SECS}s"
    "${RUN_TEST_CMD[@]}"
  )
fi

set +e
"${RUNNER_CMD[@]}" &
RUNNER_PID=$!
START_TS="$(date +%s)"

while kill -0 "${RUNNER_PID}" 2>/dev/null; do
  NOW_TS="$(date +%s)"
  ELAPSED="$((NOW_TS - START_TS))"
  SNAPSHOT=""
  if command -v ps >/dev/null 2>&1; then
    SNAPSHOT="$(
      ps -Ao pid,ppid,etime,pcpu,pmem,command 2>/dev/null |
        awk -v pid="${RUNNER_PID}" '$1 == pid || $2 == pid {print}' |
        head -n 3 |
        tr '\n' ';'
    )"
  fi
  echo "[ocr-probe][heartbeat] elapsed=${ELAPSED}s runner_pid=${RUNNER_PID} procs=${SNAPSHOT}"
  sleep "${XIUXIAN_OCR_PROBE_HEARTBEAT_SECS}"
done

wait "${RUNNER_PID}"
EXIT_CODE=$?
set -e

if [[ ${EXIT_CODE} -eq 124 ]]; then
  echo "[ocr-probe] HARD TIMEOUT: probe exceeded ${XIUXIAN_OCR_PROBE_HARD_TIMEOUT_SECS}s and was terminated" >&2
fi
exit "${EXIT_CODE}"
