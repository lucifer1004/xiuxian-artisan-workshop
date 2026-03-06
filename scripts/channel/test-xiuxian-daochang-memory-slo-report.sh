#!/usr/bin/env bash
# Wrapper for the unified memory/session SLO report.
# Keeps argument orchestration out of justfile.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

evolution_report_json="${1:-.run/reports/xiuxian-daochang-memory-evolution.json}"
benchmark_report_json="${2:-.run/reports/xiuxian-daochang-memory-benchmark.json}"
session_matrix_report_json="${3:-.run/reports/agent-channel-session-matrix.json}"
runtime_log_file="${4:-}"
enable_stream_gate="${5:-false}"
output_json="${6:-.run/reports/xiuxian-daochang-memory-slo-report.json}"
output_markdown="${7:-.run/reports/xiuxian-daochang-memory-slo-report.md}"
shift $(($# < 7 ? $# : 7))

args=(
  --evolution-report-json "${evolution_report_json}"
  --benchmark-report-json "${benchmark_report_json}"
  --session-matrix-report-json "${session_matrix_report_json}"
  --output-json "${output_json}"
  --output-markdown "${output_markdown}"
)

if [ -n "${runtime_log_file}" ]; then
  args+=(--runtime-log-file "${runtime_log_file}")
fi
if [ "${enable_stream_gate}" = "true" ]; then
  args+=(--enable-stream-gate)
fi

exec python3 "${SCRIPT_DIR}/test_xiuxian_daochang_memory_slo_report.py" "${args[@]}" "$@"
