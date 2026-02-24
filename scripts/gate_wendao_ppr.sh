#!/usr/bin/env bash
set -euo pipefail

MATRIX_FILE="${1:-docs/testing/wendao-query-regression-matrix.json}"
LIMIT="${2:-10}"
PROFILE="${3:-debug}"       # debug | release
BUILD_MODE="${4:-no-build}" # no-build | build
MIN_TOP3_RATE="${5:-1.0}"
STEM="${6:-README}"
RUNS="${7:-5}"
WARM_RUNS="${8:-1}"
SUBGRAPH_MODE="${9:-auto}" # auto | disabled | force
MAX_P95_MS="${10:-1500}"
MAX_AVG_MS="${11:-1200}"
EXPECT_SUBGRAPH_COUNT_MIN="${12:-1}"
OUTPUT_MODE="${13:-text}" # text | json

BINARY_PATH="${XIUXIAN_WENDAO_GATE_BINARY:-}"
CONFIG_PATH="${XIUXIAN_WENDAO_GATE_CONFIG:-}"
REPORT_DIR="${XIUXIAN_WENDAO_GATE_REPORT_DIR:-}"
QUERY_PREFIX="${XIUXIAN_WENDAO_GATE_QUERY_PREFIX:-}"
EVAL_SCRIPT="${XIUXIAN_WENDAO_GATE_EVAL_SCRIPT:-scripts/evaluate_wendao_retrieval.py}"
BENCH_SCRIPT="${XIUXIAN_WENDAO_GATE_BENCH_SCRIPT:-scripts/benchmark_wendao_related.py}"
GATE_LINE_SCRIPT="${XIUXIAN_WENDAO_GATE_LINE_SCRIPT:-scripts/render_wendao_gate_line.py}"

case "${PROFILE}" in
debug | release) ;;
*)
  echo "Invalid profile: ${PROFILE} (expected: debug|release)" >&2
  exit 2
  ;;
esac

case "${BUILD_MODE}" in
no-build | build) ;;
*)
  echo "Invalid build mode: ${BUILD_MODE} (expected: no-build|build)" >&2
  exit 2
  ;;
esac

case "${SUBGRAPH_MODE}" in
auto | disabled | force) ;;
*)
  echo "Invalid subgraph mode: ${SUBGRAPH_MODE} (expected: auto|disabled|force)" >&2
  exit 2
  ;;
esac

case "${OUTPUT_MODE}" in
text | json) ;;
*)
  echo "Invalid output mode: ${OUTPUT_MODE} (expected: text|json)" >&2
  exit 2
  ;;
esac

if [ -z "${BINARY_PATH}" ]; then
  if [ -x "target/${PROFILE}/wendao" ]; then
    BINARY_PATH="target/${PROFILE}/wendao"
  elif [ -x ".cache/target-codex-wendao/${PROFILE}/wendao" ]; then
    BINARY_PATH=".cache/target-codex-wendao/${PROFILE}/wendao"
  elif [ "${BUILD_MODE}" = "no-build" ]; then
    echo "No wendao binary found for profile=${PROFILE}; set XIUXIAN_WENDAO_GATE_BINARY or use build mode." >&2
    exit 2
  fi
fi

eval_cmd=(
  uv run python "${EVAL_SCRIPT}"
  --root .
  --matrix-file "${MATRIX_FILE}"
  --limit "${LIMIT}"
  --min-top3-rate "${MIN_TOP3_RATE}"
)
if [ -n "${QUERY_PREFIX}" ]; then
  eval_cmd+=(--query-prefix "${QUERY_PREFIX}")
fi

bench_cmd=(
  uv run python "${BENCH_SCRIPT}"
  --root .
  --stem "${STEM}"
  --runs "${RUNS}"
  --warm-runs "${WARM_RUNS}"
  --ppr-subgraph-mode "${SUBGRAPH_MODE}"
  --max-p95-ms "${MAX_P95_MS}"
  --max-avg-ms "${MAX_AVG_MS}"
  --expect-subgraph-count-min "${EXPECT_SUBGRAPH_COUNT_MIN}"
)

if [ "${PROFILE}" = "release" ]; then
  eval_cmd+=(--release)
  bench_cmd+=(--release)
fi

if [ "${BUILD_MODE}" = "no-build" ]; then
  eval_cmd+=(--no-build)
  bench_cmd+=(--no-build)
fi

if [ -n "${BINARY_PATH}" ]; then
  eval_cmd+=(--binary "${BINARY_PATH}")
  bench_cmd+=(--binary "${BINARY_PATH}")
fi

if [ -n "${CONFIG_PATH}" ]; then
  eval_cmd+=(--config "${CONFIG_PATH}")
  bench_cmd+=(--config "${CONFIG_PATH}")
fi

if [ "${OUTPUT_MODE}" = "json" ]; then
  eval_cmd+=(--json)
  bench_cmd+=(--json)
fi

if [ "${OUTPUT_MODE}" = "text" ]; then
  echo "[gate-wendao-ppr] retrieval quality gate"
  "${eval_cmd[@]}"
  echo
  echo "[gate-wendao-ppr] related PPR benchmark gate"
  "${bench_cmd[@]}"
else
  tmp_dir="$(mktemp -d)"
  cleanup() {
    rm -rf "${tmp_dir}"
  }
  trap cleanup EXIT

  retrieval_output="${tmp_dir}/retrieval_eval.json"
  related_output="${tmp_dir}/related_benchmark.json"

  eval_rc=0
  if "${eval_cmd[@]}" >"${retrieval_output}"; then
    eval_rc=0
  else
    eval_rc=$?
  fi
  related_rc=0
  if "${bench_cmd[@]}" >"${related_output}"; then
    related_rc=0
  else
    related_rc=$?
  fi

  gate_line="$(
    uv run python "${GATE_LINE_SCRIPT}" \
      --retrieval-report "${retrieval_output}" \
      --related-report "${related_output}" \
      --min-top3-rate "${MIN_TOP3_RATE}" \
      --retrieval-exit-code "${eval_rc}" \
      --related-exit-code "${related_rc}"
  )"
  echo "${gate_line}" >&2

  if [ -n "${REPORT_DIR}" ]; then
    mkdir -p "${REPORT_DIR}"
    retrieval_report="${REPORT_DIR}/retrieval_eval.json"
    related_report="${REPORT_DIR}/related_benchmark.json"
    echo "[gate-wendao-ppr] retrieval quality gate -> ${retrieval_report}" >&2
    cp "${retrieval_output}" "${retrieval_report}"
    echo "[gate-wendao-ppr] related PPR benchmark gate -> ${related_report}" >&2
    cp "${related_output}" "${related_report}"
  else
    echo "[gate-wendao-ppr] retrieval quality gate" >&2
    cat "${retrieval_output}"
    echo "[gate-wendao-ppr] related PPR benchmark gate" >&2
    cat "${related_output}"
  fi

  if [ "${eval_rc}" -ne 0 ] || [ "${related_rc}" -ne 0 ]; then
    exit 1
  fi
fi
