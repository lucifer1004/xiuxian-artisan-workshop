#!/usr/bin/env bash
set -euo pipefail

MATRIX_FILE="${1:-docs/testing/wendao-query-regression-matrix.json}"
LIMIT="${2:-10}"
PROFILE="${3:-debug}"       # debug | release
BUILD_MODE="${4:-no-build}" # no-build | build
MIN_TOP3_RATE="${5:-0.0}"
OUTPUT_MODE="${6:-text}" # text | json

cmd=(
  uv run python scripts/evaluate_wendao_retrieval.py
  --root .
  --matrix-file "${MATRIX_FILE}"
  --limit "${LIMIT}"
  --min-top3-rate "${MIN_TOP3_RATE}"
)

case "${PROFILE}" in
debug) ;;
release) cmd+=(--release) ;;
*)
  echo "Invalid profile: ${PROFILE} (expected: debug|release)" >&2
  exit 2
  ;;
esac

case "${BUILD_MODE}" in
no-build) cmd+=(--no-build) ;;
build) ;;
*)
  echo "Invalid build mode: ${BUILD_MODE} (expected: no-build|build)" >&2
  exit 2
  ;;
esac

case "${OUTPUT_MODE}" in
text) ;;
json) cmd+=(--json) ;;
*)
  echo "Invalid output mode: ${OUTPUT_MODE} (expected: text|json)" >&2
  exit 2
  ;;
esac

"${cmd[@]}"
