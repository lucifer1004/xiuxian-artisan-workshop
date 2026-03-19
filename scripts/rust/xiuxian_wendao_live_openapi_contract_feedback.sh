#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo_bin="${CARGO_BIN:-${script_dir}/cargo_exec.sh}"
target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

# Live OpenAPI artifact lane:
# - bundled Wendao gateway OpenAPI stays aligned to the declared route inventory
# - qianji rest_docs helper accepts the real bundled Wendao artifact without findings
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-wendao --lib \
  bundled_gateway_openapi_document_

CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-qianji \
  --test wendao_live_rest_docs_contract_feedback
