#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-deterministic}" # deterministic | network
RUNS_INPUT="${2:-}"

shift_count=0
if [ "${#}" -ge 1 ]; then
  shift_count=1
fi
if [ "${#}" -ge 2 ]; then
  shift_count=2
fi
if [ "${shift_count}" -gt 0 ]; then
  shift "${shift_count}"
fi

case "${MODE}" in
deterministic)
  DEFAULT_RUNS="3"
  ;;
network)
  DEFAULT_RUNS="5"
  ;;
*)
  echo "Invalid mode: ${MODE} (expected: deterministic|network)" >&2
  exit 2
  ;;
esac

RUNS="${RUNS_INPUT:-${DEFAULT_RUNS}}"

cmd=(
  uv run python scripts/benchmark_skills_tools.py
  --runs "${RUNS}"
  --json
  --snapshot-default-metric p50
  --snapshot-network-metric trimmed_avg
)

case "${MODE}" in
deterministic)
  cmd+=(
    --crawl4ai-scenarios local
    --snapshot-gate-scope deterministic
    --enforce-cli-ordering
    --cli-ordering-tolerance-ms 50
    --strict-snapshot
  )
  ;;
network)
  cmd+=(
    --tools crawl4ai.crawl_url
    --crawl4ai-scenarios both
    --snapshot-gate-scope all
  )
  ;;
esac

if [ "${#}" -gt 0 ]; then
  cmd+=("$@")
fi

if [[ ${OMNI_SKILLS_TOOLS_GATE_DRY_RUN:-} == "1" || ${OMNI_SKILLS_TOOLS_GATE_DRY_RUN:-} == "true" ]]; then
  printf '%q ' "${cmd[@]}"
  printf '\n'
  exit 0
fi

"${cmd[@]}"
