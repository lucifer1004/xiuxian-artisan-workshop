#!/usr/bin/env bash
set -euo pipefail

port="${1:-6379}"
valkey_url="${2:-redis://127.0.0.1:${port}/0}"

cleanup() {
  bash scripts/channel/valkey-stop.sh "${port}" || true
}
trap cleanup EXIT

bash scripts/channel/valkey-start.sh "${port}"
bash scripts/channel/test-omni-agent-valkey-full.sh "${valkey_url}"
