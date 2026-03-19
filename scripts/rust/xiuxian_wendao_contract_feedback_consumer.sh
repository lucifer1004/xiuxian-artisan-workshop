#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo_bin="${CARGO_BIN:-${script_dir}/cargo_exec.sh}"
target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

# Downstream consumer lane:
# - rest_docs warning findings remain mappable to Wendao reference knowledge
# - modularity warning findings remain mappable to Wendao architecture knowledge
# - qianji persisted rest_docs flow writes Wendao-native entries through a sink
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-wendao --lib \
  contract_feedback_adapter_

CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-qianji \
  --test wendao_persisted_rest_docs_contract_feedback
