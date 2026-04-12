#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${PRJ_ROOT:-${DEVENV_ROOT:-$(cd "${SCRIPT_DIR}/../.." && pwd)}}"
PYTHON_BIN="${WENDAO_FRONTEND_PYTHON:-}"

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

PIDFILE="${WENDAO_FRONTEND_PIDFILE:-$PROJECT_ROOT/.run/wendao-frontend/wendao-frontend.pid}"
HOST="${WENDAO_FRONTEND_HOST:-127.0.0.1}"
PORT="${WENDAO_FRONTEND_PORT:-9518}"
TIMEOUT_SECS="${WENDAO_FRONTEND_HEALTH_TIMEOUT_SECS:-2}"

"$PYTHON_BIN" "$PROJECT_ROOT/scripts/channel/check_wendao_frontend_health.py" \
  --host "$HOST" \
  --port "$PORT" \
  --pidfile "$PIDFILE" \
  --timeout-secs "$TIMEOUT_SECS"
