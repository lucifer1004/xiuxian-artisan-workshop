#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo_bin="${CARGO_BIN:-${script_dir}/cargo_exec.sh}"
target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

# LLM runtime role boundary:
# - minimax provider setting should resolve minimax OpenAI-compatible URL.
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-daochang --bin xiuxian-daochang \
  resolve_runtime_inference_url_uses_minimax_provider_default_when_configured

# Embedding runtime role boundary:
# - gateway embedding guard must preserve an explicit http backend selection.
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-daochang --lib \
  gateway_preserves_configured_http_embedding_backend

# Backend parsing contracts:
# - embedding parser recognizes the active http/openai_http/litellm_rs families.
CARGO_TARGET_DIR="${target_dir}" "${cargo_bin}" test -p xiuxian-llm --test unit_test \
  embedding_backend -- --nocapture
