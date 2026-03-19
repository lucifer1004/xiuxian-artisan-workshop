#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo_bin="${CARGO_BIN:-${script_dir}/cargo_exec.sh}"
target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

if ! command -v cargo-nextest >/dev/null 2>&1; then
  echo "cargo-nextest is required but not installed." >&2
  echo "Install with: nix profile add nixpkgs#cargo-nextest" >&2
  exit 1
fi

# Scenario-audit contract lane:
# - qianji contract-feedback export path
# - formal-audit advisory bridge
# - scenario snapshots
# - pinned audit-flow XSD contract
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" nextest run -p xiuxian-qianji \
  --test contract_feedback_pipeline \
  --test test_formal_audit_advisory_executor \
  --test scenarios_test \
  --test audit_scenario_contract \
  --no-fail-fast
