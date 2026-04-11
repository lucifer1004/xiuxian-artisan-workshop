#!/usr/bin/env bash
set -euo pipefail

profile="${1:-safe}"
model_root="${2:-}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

run_embedding_warmup_safe() {
  local guard_label guard_max_rss guard_growth
  local embed_text
  guard_label="${LOCAL_MODEL_EMBED_GUARD_LABEL:-local-model-embed}"
  guard_max_rss="${LOCAL_MODEL_EMBED_MAX_RSS_GB:-10}"
  guard_growth="${LOCAL_MODEL_EMBED_MAX_GROWTH_GB_PER_MIN:-0}"
  embed_text="${LOCAL_MODEL_EMBED_TEXT:-local model embedding warmup}"

  local cmd=(
    "${PROJECT_ROOT}/scripts/rust/cargo_exec.sh"
    run
    -p
    xiuxian-daochang
    --no-default-features
  )
  cmd+=(-- embedding-warmup --text "${embed_text}")

  UV_CACHE_DIR="${UV_CACHE_DIR:-.cache/uv}" \
    uv run python "${PROJECT_ROOT}/scripts/guarded_nextest.py" \
    --label "${guard_label}" \
    --max-rss-gb "${guard_max_rss}" \
    --max-growth-gb-per-min "${guard_growth}" \
    --max-pids "${LOCAL_MODEL_EMBED_MAX_PIDS:-0}" \
    --poll-ms "${LOCAL_MODEL_EMBED_POLL_MS:-500}" \
    --grace-ms "${LOCAL_MODEL_EMBED_GRACE_MS:-1500}" \
    --log-every "${LOCAL_MODEL_EMBED_LOG_EVERY:-4}" \
    --log-file "${LOCAL_MODEL_EMBED_LOG_FILE:-.run/logs/local-model-embed-guard.log}" \
    --report-json "${LOCAL_MODEL_EMBED_REPORT_JSON:-.run/reports/local-model-safe/embed-latest.json}" \
    --samples-jsonl "${LOCAL_MODEL_EMBED_SAMPLES_JSONL:-.run/reports/local-model-safe/embed-samples.jsonl}" \
    --history-jsonl "${LOCAL_MODEL_EMBED_HISTORY_JSONL:-.run/reports/local-model-safe/embed-history.jsonl}" \
    -- "${cmd[@]}"
}

run_vision_safe() {
  bash "${PROJECT_ROOT}/scripts/rust/test_vision_smoke_lane.sh"
}

run_vision_full() {
  bash "${PROJECT_ROOT}/scripts/rust/test_vision_heavy_lane.sh" "${model_root}"
}

case "${profile}" in
safe)
  run_embedding_warmup_safe
  run_vision_safe
  ;;
full)
  run_embedding_warmup_safe
  run_vision_full
  ;;
embed-only)
  run_embedding_warmup_safe
  ;;
vision-only)
  run_vision_safe
  ;;
vision-heavy-only)
  run_vision_full
  ;;
*)
  cat <<'USAGE' >&2
Usage: scripts/rust/test_local_models_safe.sh [safe|full|embed-only|vision-only|vision-heavy-only] [model_root]
USAGE
  exit 2
  ;;
esac
