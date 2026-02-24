#!/usr/bin/env bash
set -euo pipefail

BASE_REPORT_DIR="${1:-.run/reports/wendao-ppr-gate}"
MIXED_REPORT_DIR="${2:-.run/reports/wendao-ppr-mixed-canary}"
REQUIRED_RUNS="${3:-${XIUXIAN_WENDAO_ROLLOUT_REQUIRED_RUNS:-7}}"
REQUIRED_MIXED_TOP3_RATE="${4:-${XIUXIAN_WENDAO_ROLLOUT_MIN_MIXED_TOP3_RATE:-0.9}}"
STRICT_READY="${5:-${XIUXIAN_WENDAO_ROLLOUT_STRICT_READY:-0}}"
MIN_BASE_TOP3_RATE="${XIUXIAN_WENDAO_GATE_MIN_TOP3_RATE:-1.0}"
STRICT_GATE_SUMMARY="${XIUXIAN_WENDAO_GATE_SUMMARY_STRICT_GREEN:-0}"
RUNNER_OS_LABEL="${RUNNER_OS:-local}"

VALIDATION_REPORT_PATH="${BASE_REPORT_DIR}/report_validation.json"
ROLL_OUT_STATUS_JSON="${BASE_REPORT_DIR}/wendao_rollout_status.json"
ROLL_OUT_STATUS_MD="${BASE_REPORT_DIR}/wendao_rollout_status.md"
GATE_STATUS_JSON="${BASE_REPORT_DIR}/wendao_gate_status_summary.json"
GATE_STATUS_MD="${BASE_REPORT_DIR}/wendao_gate_status_summary.md"
GATE_ROLLOUT_STATUS_MD="${BASE_REPORT_DIR}/wendao_gate_rollout_status.md"
PREVIOUS_STATUS_JSON="${BASE_REPORT_DIR}/wendao_rollout_status.previous.json"
PREVIOUS_FETCH_REPORT="${BASE_REPORT_DIR}/wendao_rollout_status_previous_fetch.json"

FETCH_REMOTE_STATUS="${XIUXIAN_WENDAO_ROLLOUT_FETCH_REMOTE_STATUS:-1}"
REMOTE_WORKFLOW_FILE="${XIUXIAN_WENDAO_ROLLOUT_REMOTE_WORKFLOW_FILE:-ci.yaml}"
REMOTE_ARTIFACT_NAME="${XIUXIAN_WENDAO_ROLLOUT_REMOTE_ARTIFACT_NAME:-}"
REMOTE_RUN_STATUS="${XIUXIAN_WENDAO_ROLLOUT_REMOTE_RUN_STATUS:-completed}"
ROLLOUT_LINE_SCRIPT="${XIUXIAN_WENDAO_ROLLOUT_LINE_SCRIPT:-scripts/render_wendao_rollout_gate_line.py}"

is_truthy() {
  case "${1:-}" in
  1 | true | TRUE | yes | YES | on | ON)
    return 0
    ;;
  *)
    return 1
    ;;
  esac
}

mkdir -p "${BASE_REPORT_DIR}"

if [ ! -f "${PREVIOUS_STATUS_JSON}" ] && is_truthy "${FETCH_REMOTE_STATUS}" && [ -n "${REMOTE_ARTIFACT_NAME}" ]; then
  uv run python scripts/fetch_previous_skills_benchmark_artifact.py \
    --artifact-name "${REMOTE_ARTIFACT_NAME}" \
    --workflow-file "${REMOTE_WORKFLOW_FILE}" \
    --run-status "${REMOTE_RUN_STATUS}" \
    --preferred-member "wendao_rollout_status.json" \
    --fallback-member "wendao_rollout_status.json" \
    --output "${PREVIOUS_STATUS_JSON}" \
    >"${PREVIOUS_FETCH_REPORT}"
fi

uv run python scripts/validate_wendao_gate_reports.py \
  --root . \
  --report-dir "${BASE_REPORT_DIR}" \
  --mixed-report-dir "${MIXED_REPORT_DIR}" \
  --json >"${VALIDATION_REPORT_PATH}"

gate_summary_cmd=(
  uv run python scripts/render_wendao_gate_status_summary.py
  --base-report-dir "${BASE_REPORT_DIR}"
  --mixed-report-dir "${MIXED_REPORT_DIR}"
  --min-base-top3-rate "${MIN_BASE_TOP3_RATE}"
  --min-mixed-top3-rate "${REQUIRED_MIXED_TOP3_RATE}"
  --runner-os "${RUNNER_OS_LABEL}"
  --output-json "${GATE_STATUS_JSON}"
  --output-markdown "${GATE_STATUS_MD}"
)
if is_truthy "${STRICT_GATE_SUMMARY}"; then
  gate_summary_cmd+=(--strict-green)
fi
"${gate_summary_cmd[@]}" >&2

rollout_cmd=(
  uv run python scripts/render_wendao_ppr_rollout_status.py
  --base-report-dir "${BASE_REPORT_DIR}"
  --mixed-report-dir "${MIXED_REPORT_DIR}"
  --validation-report "${VALIDATION_REPORT_PATH}"
  --previous-status-json "${PREVIOUS_STATUS_JSON}"
  --required-consecutive-runs "${REQUIRED_RUNS}"
  --required-mixed-top3-rate "${REQUIRED_MIXED_TOP3_RATE}"
  --output-json "${ROLL_OUT_STATUS_JSON}"
  --output-markdown "${ROLL_OUT_STATUS_MD}"
)
strict_ready_enabled=0
if is_truthy "${STRICT_READY}"; then
  strict_ready_enabled=1
fi
"${rollout_cmd[@]}"

status_line="$(
  uv run python "${ROLLOUT_LINE_SCRIPT}" \
    --status-json "${ROLL_OUT_STATUS_JSON}"
)"

IFS=$'\t' read -r ready_flag gate_line <<<"${status_line}"
echo "${gate_line}" >&2

{
  if [ -f "${GATE_STATUS_MD}" ]; then
    cat "${GATE_STATUS_MD}"
    echo
  fi
  if [ -f "${ROLL_OUT_STATUS_MD}" ]; then
    cat "${ROLL_OUT_STATUS_MD}"
  fi
} >"${GATE_ROLLOUT_STATUS_MD}"

cat "${ROLL_OUT_STATUS_JSON}"

if [ "${strict_ready_enabled}" -eq 1 ] && [ "${ready_flag}" != "1" ]; then
  exit 1
fi
