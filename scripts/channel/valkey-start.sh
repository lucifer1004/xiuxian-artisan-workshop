#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
source "${SCRIPT_DIR}/valkey-common.sh"

resolve_valkey_field() {
  uv run python "${PROJECT_ROOT}/scripts/channel/resolve_valkey_endpoint.py" --field "$1"
}

DEFAULT_PORT="$(resolve_valkey_field port)"
DEFAULT_HOST="$(resolve_valkey_field host)"
DEFAULT_DB="$(resolve_valkey_field db)"

PORT="${1:-${VALKEY_PORT:-${DEFAULT_PORT}}}"
HOST="${VALKEY_HOST:-${DEFAULT_HOST}}"
DB="${VALKEY_DB:-${DEFAULT_DB}}"

if ! command -v valkey-server >/dev/null 2>&1; then
  echo "Error: valkey-server not found in PATH." >&2
  exit 1
fi
if ! command -v valkey-cli >/dev/null 2>&1; then
  echo "Error: valkey-cli not found in PATH." >&2
  exit 1
fi

RUNTIME_DIR="${PRJ_RUNTIME_DIR:-.run}/valkey"
DATA_DIR="${PRJ_CACHE_HOME:-.cache}/valkey"
mkdir -p "$RUNTIME_DIR" "$DATA_DIR"
PIDFILE="$RUNTIME_DIR/valkey-${PORT}.pid"
LOGFILE="$RUNTIME_DIR/valkey-${PORT}.log"
URL="redis://${HOST}:${PORT}/${DB}"

if valkey_listener_matches_pidfile "$PIDFILE" "$URL" && valkey-cli -u "$URL" ping >/dev/null 2>&1; then
  echo "Valkey already running on ${PORT} (pid $(cat "$PIDFILE"))."
  exit 0
fi

if valkey-cli -u "$URL" ping >/dev/null 2>&1; then
  echo "Error: Valkey is reachable at $URL but pidfile $PIDFILE does not match the listener." >&2
  exit 1
fi

echo "Starting Valkey on port ${PORT}..."
valkey-server \
  --port "$PORT" \
  --bind "${HOST}" \
  --daemonize yes \
  --dir "$DATA_DIR" \
  --pidfile "$PIDFILE" \
  --logfile "$LOGFILE"

for _ in $(seq 1 50); do
  if valkey_listener_matches_pidfile "$PIDFILE" "$URL" && valkey-cli -u "$URL" ping >/dev/null 2>&1; then
    echo "Valkey started. pidfile=$PIDFILE logfile=$LOGFILE datadir=$DATA_DIR"
    exit 0
  fi
  sleep 0.1
done

echo "Error: Valkey did not become healthy at $URL." >&2
exit 1
