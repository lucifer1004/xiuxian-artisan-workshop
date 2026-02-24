#!/usr/bin/env bash
# Run Telegram channel in polling mode with local Valkey bootstrapping.
# Usage: TELEGRAM_BOT_TOKEN=xxx ./scripts/channel/agent-channel-polling.sh [valkey_port]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${PROJECT_ROOT}"

# Source .env if present
if [ -f .env ]; then
  set -a
  # shellcheck source=/dev/null
  source .env
  set +a
fi

VALKEY_PORT="${VALKEY_PORT:-6379}"
if [ $# -gt 0 ] && [[ $1 =~ ^[0-9]+$ ]]; then
  VALKEY_PORT="$1"
  shift
fi

bash "${SCRIPT_DIR}/valkey-start.sh" "${VALKEY_PORT}"
export VALKEY_URL="${VALKEY_URL:-redis://127.0.0.1:${VALKEY_PORT}/0}"

echo "Starting Telegram channel (polling mode)..."
echo "VALKEY_URL='${VALKEY_URL}'"
echo "Telegram ACL source='.config/omni-dev-fusion/settings.yaml (telegram.acl.*)'"

cargo run -p omni-agent -- channel \
  --mode polling \
  "$@"
