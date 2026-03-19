#!/usr/bin/env bash
set -euo pipefail

timeout_secs="${1:-3600}"

just rust-lint-inheritance-check
just rust-test-layout
just rust-check "${timeout_secs}"
just rust-clippy
just rust-nextest
just rust-xiuxian-llm-mcp
just rust-xiuxian-daochang-mcp-facade-smoke
just rust-xiuxian-daochang-backend-role-contracts
just rust-xiuxian-qianji-scenario-audit-contracts
just rust-xiuxian-testing-contract-gates
just rust-xiuxian-wendao-contract-feedback-consumer
if [[ ${OMNI_ENABLE_EMBED_ROLE_PERF_GATE:-0} == "1" ]]; then
  profile="${OMNI_EMBED_ROLE_PERF_GATE_PROFILE:-medium}"
  case "${profile}" in
  medium)
    just rust-xiuxian-daochang-embedding-role-perf-medium-gate
    ;;
  heavy)
    just rust-xiuxian-daochang-embedding-role-perf-heavy-gate
    ;;
  *)
    echo "Unsupported OMNI_EMBED_ROLE_PERF_GATE_PROFILE='${profile}' (expected: medium|heavy)." >&2
    exit 1
    ;;
  esac
fi
just rust-test-xiuxian-core-rs
just rust-security-gate
