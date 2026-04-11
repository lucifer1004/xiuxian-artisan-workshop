#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${PRJ_ROOT:-${DEVENV_ROOT:-$(cd "${SCRIPT_DIR}/../.." && pwd)}}"
PYTHON_BIN="${WENDAO_GATEWAY_PYTHON:-}"

if [ -z "$PYTHON_BIN" ]; then
  PYTHON_BIN="${PYO3_PYTHON:-}"
fi
if [ -z "$PYTHON_BIN" ]; then
  PYTHON_BIN="$(command -v python3 2>/dev/null || true)"
fi
if [ -z "$PYTHON_BIN" ]; then
  echo "Error: python3 not found in PATH." >&2
  exit 1
fi

PIDFILE="${WENDAO_GATEWAY_PIDFILE:-$PROJECT_ROOT/.run/wendao-gateway/wendao.pid}"
CONFIG_PATH="${WENDAO_GATEWAY_CONFIG:-$PROJECT_ROOT/wendao.toml}"
HOST="${WENDAO_GATEWAY_HOST:-127.0.0.1}"
TIMEOUT_SECS="${WENDAO_GATEWAY_HEALTH_TIMEOUT_SECS:-2}"

PORT="$("$PYTHON_BIN" "$PROJECT_ROOT/scripts/channel/resolve_wendao_gateway_port.py" --config "$CONFIG_PATH")"

"$PYTHON_BIN" "$PROJECT_ROOT/scripts/channel/check_wendao_gateway_health.py" \
  --host "$HOST" \
  --port "$PORT" \
  --pidfile "$PIDFILE" \
  --timeout-secs "$TIMEOUT_SECS"
