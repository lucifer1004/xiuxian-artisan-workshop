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

# Contract-kernel lane:
# - kernel model and report invariants
# - executable suite runner semantics
# - deterministic rest_docs and modularity packs
# - Wendao export envelope projection
# - docs kernel governance surface
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" nextest run -p xiuxian-testing \
  --test contracts_kernel \
  --test contracts_rest_docs \
  --test contracts_modularity \
  --test contracts_runner \
  --test contracts_knowledge_export \
  --test docs_kernel_contract \
  --no-fail-fast
