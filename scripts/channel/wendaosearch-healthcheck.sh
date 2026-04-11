#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${PRJ_ROOT:-${DEVENV_ROOT:-$(cd "${SCRIPT_DIR}/../.." && pwd)}}"
source "${SCRIPT_DIR}/wendaosearch-common.sh"
source "${SCRIPT_DIR}/process-runtime.sh"

HOST="$(wendaosearch_effective_host "$PROJECT_ROOT")"
PORT="$(wendaosearch_effective_port "$PROJECT_ROOT")"
PIDFILE="$(wendaosearch_resolve_path "$PROJECT_ROOT" "$(wendaosearch_effective_pidfile)")"

if ! managed_listener_matches_pidfile "$PIDFILE" "$PORT"; then
  if [ -n "$(managed_listener_pid "$PORT")" ]; then
    echo "Error: WendaoSearch is reachable on ${HOST}:${PORT} but pidfile ${PIDFILE} does not match the listener." >&2
  else
    echo "Error: WendaoSearch is not listening on ${HOST}:${PORT}." >&2
  fi
  exit 1
fi
